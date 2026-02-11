use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tauri::Emitter;
use tauri::Manager;
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::commands::{get_gpu_status, list_cuda_devices, GpuStatus, ListCudaDevicesResponse};
use crate::state::AppState;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:48991";
const WORKER_RUN_DOUBLE_MA_PATH: &str = "/v1/double_ma/run";
const WORKER_CANCEL_PATH: &str = "/v1/cancel";

#[derive(Default)]
pub struct DistributedState {
    worker_server: Mutex<Option<WorkerServerHandle>>,
}

struct WorkerServerHandle {
    bind_addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    join: tauri::async_runtime::JoinHandle<()>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerServerConfig {
    #[serde(default)]
    pub bind_addr: String,
    pub auth_token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerServerStatus {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_addr: Option<String>,
}

#[derive(Clone)]
struct WorkerAppState {
    app: tauri::AppHandle,
    token: String,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    ok: bool,
    version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct CapabilitiesResponse {
    gpu_status: GpuStatus,
    cuda_devices: ListCudaDevicesResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerDoubleMaRunRequest {
    csv_path: String,
    req: ta_app_core::DoubleMaRequest,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerDoubleMaRunResponse {
    csv_path: String,
    cache_hit: bool,
    cache_path: String,
    result: ta_app_core::BackendOptimizationResult,
}

#[derive(Debug, Clone, Serialize)]
struct CancelResponse {
    ok: bool,
}

fn header_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let h = headers.get(axum::http::header::AUTHORIZATION)?;
    let s = h.to_str().ok()?;
    let s = s.trim();
    let rest = s.strip_prefix("Bearer ")?;
    Some(rest.trim())
}

fn ensure_auth(expected: &str, headers: &HeaderMap) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let want = expected.trim();
    if want.is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "worker auth token not configured".to_string(),
            }),
        ));
    }
    let got = header_bearer_token(headers).unwrap_or("");
    if got == want {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".to_string(),
            }),
        ))
    }
}

async fn worker_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn worker_capabilities(
    State(st): State<WorkerAppState>,
    headers: HeaderMap,
) -> Result<Json<CapabilitiesResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_auth(&st.token, &headers)?;
    let gpu_status = get_gpu_status().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;
    let cuda_devices = list_cuda_devices().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;
    Ok(Json(CapabilitiesResponse {
        gpu_status,
        cuda_devices,
    }))
}

async fn worker_cancel(
    State(st): State<WorkerAppState>,
    headers: HeaderMap,
) -> Result<Json<CancelResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_auth(&st.token, &headers)?;
    let state = st.app.state::<AppState>();
    state.request_cancel();
    Ok(Json(CancelResponse { ok: true }))
}

async fn worker_run_double_ma(
    State(st): State<WorkerAppState>,
    headers: HeaderMap,
    Json(payload): Json<WorkerDoubleMaRunRequest>,
) -> Result<Json<WorkerDoubleMaRunResponse>, (StatusCode, Json<ErrorResponse>)> {
    ensure_auth(&st.token, &headers)?;

    let state = st.app.state::<AppState>();
    let _run_guard = state
        .try_begin_run()
        .map_err(|e| (StatusCode::CONFLICT, Json(ErrorResponse { error: e })))?;

    let cancel = state.cancel_token();
    let base = st
        .app
        .path()
        .app_local_data_dir()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;
    let cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let csv_path = payload.csv_path.trim().to_string();
    if csv_path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "csv_path is empty".to_string(),
            }),
        ));
    }

    let req = payload.req;
    let cancel2 = cancel.clone();
    let cache_dir2 = cache_dir.clone();
    let csv_path2 = csv_path.clone();
    let (result, cache_hit, cache_path) = tauri::async_runtime::spawn_blocking(move || {
        let outcome = crate::data_cache::load_or_build(Path::new(&csv_path2), &cache_dir2)?;
        let candles = outcome.candles;
        let res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            None,
        )?;
        Ok::<_, String>((res, outcome.cache_hit, outcome.cache_path))
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?
    .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e })))?;

    Ok(Json(WorkerDoubleMaRunResponse {
        csv_path,
        cache_hit,
        cache_path: cache_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        result,
    }))
}

fn status_from_lock(lock: &Mutex<Option<WorkerServerHandle>>) -> Result<WorkerServerStatus, String> {
    let guard = lock
        .lock()
        .map_err(|_| "failed to lock distributed state".to_string())?;
    let Some(handle) = guard.as_ref() else {
        return Ok(WorkerServerStatus {
            running: false,
            bind_addr: None,
        });
    };
    Ok(WorkerServerStatus {
        running: true,
        bind_addr: Some(handle.bind_addr.to_string()),
    })
}

#[tauri::command]
pub fn distributed_worker_server_status(
    dist: tauri::State<'_, DistributedState>,
) -> Result<WorkerServerStatus, String> {
    status_from_lock(&dist.worker_server)
}

#[tauri::command]
pub async fn distributed_start_worker_server(
    cfg: WorkerServerConfig,
    dist: tauri::State<'_, DistributedState>,
    app: tauri::AppHandle,
) -> Result<WorkerServerStatus, String> {
    {
        let guard = dist
            .worker_server
            .lock()
            .map_err(|_| "failed to lock distributed state".to_string())?;
        if guard.is_some() {
            drop(guard);
            return status_from_lock(&dist.worker_server);
        }
    }

    let bind_addr = cfg.bind_addr.trim();
    let bind_addr = if bind_addr.is_empty() {
        DEFAULT_BIND_ADDR
    } else {
        bind_addr
    };
    let addr: SocketAddr = bind_addr
        .parse()
        .map_err(|_| format!("invalid bind_addr '{bind_addr}' (expected ip:port)"))?;

    let token = cfg.auth_token.trim().to_string();
    if token.is_empty() {
        return Err("auth_token is required".to_string());
    }

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("failed to bind {addr}: {e}"))?;
    let actual = listener
        .local_addr()
        .map_err(|e| format!("failed to get bound addr: {e}"))?;

    let st = WorkerAppState {
        app: app.clone(),
        token: token.clone(),
    };

    let router = Router::new()
        .route("/health", get(worker_health))
        .route("/capabilities", get(worker_capabilities))
        .route("/v1/double_ma/run", post(worker_run_double_ma))
        .route("/v1/cancel", post(worker_cancel))
        .with_state(st);

    let (tx, rx) = oneshot::channel::<()>();
    let join = tauri::async_runtime::spawn(async move {
        let server = axum::serve(listener, router);
        let _ = server
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await;
    });

    let mut guard = dist
        .worker_server
        .lock()
        .map_err(|_| "failed to lock distributed state".to_string())?;
    *guard = Some(WorkerServerHandle {
        bind_addr: actual,
        shutdown: tx,
        join,
    });

    Ok(WorkerServerStatus {
        running: true,
        bind_addr: Some(actual.to_string()),
    })
}

#[tauri::command]
pub async fn distributed_stop_worker_server(
    dist: tauri::State<'_, DistributedState>,
) -> Result<(), String> {
    let handle = {
        let mut guard = dist
            .worker_server
            .lock()
            .map_err(|_| "failed to lock distributed state".to_string())?;
        guard.take()
    };
    let Some(handle) = handle else {
        return Ok(());
    };

    let _ = handle.shutdown.send(());
    let _ = handle.join.await;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct DistributedProgressPayload {
    pub mode: String,
    pub completed: usize,
    pub total: usize,
    pub successes: usize,
    pub failures: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistributedAssetsRunRequest {
    pub worker_urls: Vec<String>,
    pub auth_token: String,
    pub csv_paths: Vec<String>,
    pub req: ta_app_core::DoubleMaRequest,
}

#[derive(Debug, Serialize)]
pub struct DistributedAssetOutcome {
    pub csv_path: String,
    pub worker_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ta_app_core::BackendOptimizationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DistributedAssetsRunResponse {
    pub cancelled: bool,
    pub total: usize,
    pub completed: usize,
    pub successes: usize,
    pub failures: usize,
    pub results: Vec<DistributedAssetOutcome>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistributedTiledRunRequest {
    pub worker_urls: Vec<String>,
    pub auth_token: String,
    pub csv_path: String,
    pub req: ta_app_core::DoubleMaRequest,
    #[serde(default)]
    pub tiles_per_worker: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct DistributedTiledRunResponse {
    pub cancelled: bool,
    pub tiles_total: usize,
    pub tiles_completed: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ta_app_core::BackendOptimizationResult>,
}

fn normalize_worker_base_url(raw: &str) -> Result<String, String> {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return Err("worker url is empty".to_string());
    }
    if !s.contains("://") {
        s = format!("http://{s}");
    }
    while s.ends_with('/') {
        s.pop();
    }
    Ok(s)
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60 * 60 * 6))
        .build()
        .map_err(|e| e.to_string())
}

fn authz_header(token: &str) -> String {
    format!("Bearer {}", token.trim())
}

fn expand_u16_range((start, end, step): (u32, u32, u32)) -> Result<Vec<u16>, String> {
    let push_checked = |out: &mut Vec<u16>, v: u32| -> Result<(), String> {
        if v > u16::MAX as u32 {
            return Err(format!("period {v} exceeds u16::MAX"));
        }
        out.push(v as u16);
        Ok(())
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start)?;
        return Ok(out);
    }

    let step = step.max(1);
    let (lo, hi) = if start <= end { (start, end) } else { (end, start) };

    let mut out = Vec::new();
    let mut v = lo;
    loop {
        push_checked(&mut out, v)?;
        if v == hi {
            break;
        }
        match v.checked_add(step) {
            Some(next) if next > v && next <= hi => v = next,
            _ => break,
        }
    }
    if out.is_empty() {
        return Err("empty period range".to_string());
    }
    Ok(out)
}

#[tauri::command]
pub async fn distributed_run_double_ma_assets(
    dist_req: DistributedAssetsRunRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DistributedAssetsRunResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let app2 = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let token = dist_req.auth_token.trim().to_string();
        if token.is_empty() {
            return Err("auth_token is required".to_string());
        }

        let mut workers: Vec<String> = Vec::new();
        for w in dist_req.worker_urls {
            let base = normalize_worker_base_url(&w)?;
            if !workers.contains(&base) {
                workers.push(base);
            }
        }
        if workers.is_empty() {
            return Err("worker_urls is empty".to_string());
        }

        let mut csv_paths: Vec<String> = dist_req
            .csv_paths
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if csv_paths.is_empty() {
            return Err("csv_paths is empty".to_string());
        }
        let total = csv_paths.len();

        let client = http_client()?;
        let authz = authz_header(&token);

        // Health check workers up-front (fast fail).
        for w in &workers {
            let url = format!("{w}/health");
            let resp = client
                .get(&url)
                .timeout(Duration::from_secs(3))
                .send()
                .map_err(|e| format!("worker {w} health check failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("worker {w} health check failed: HTTP {}", resp.status()));
            }
        }

        let base_req = dist_req.req;
        if base_req.export_all_csv_path.is_some() {
            return Err("distributed assets mode does not support export_all_csv_path".to_string());
        }

        let queue = Arc::new(Mutex::new(VecDeque::from(std::mem::take(&mut csv_paths))));
        let (tx, rx) = std::sync::mpsc::channel::<DistributedAssetOutcome>();

        let mut threads = Vec::with_capacity(workers.len());
        for worker_url in workers.clone() {
            let queue2 = queue.clone();
            let tx2 = tx.clone();
            let cancel2 = cancel.clone();
            let client2 = client.clone();
            let authz2 = authz.clone();
            let req2 = base_req.clone();

            threads.push(std::thread::spawn(move || {
                loop {
                    if cancel2.load(Ordering::Relaxed) {
                        break;
                    }
                    let next = {
                        let mut q = match queue2.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        q.pop_front()
                    };
                    let Some(csv_path) = next else {
                        break;
                    };

                    let payload = WorkerDoubleMaRunRequest {
                        csv_path: csv_path.clone(),
                        req: req2.clone(),
                    };

                    let url = format!("{worker_url}{WORKER_RUN_DOUBLE_MA_PATH}");
                    let outcome = match client2
                        .post(&url)
                        .header(reqwest::header::AUTHORIZATION, authz2.clone())
                        .json(&payload)
                        .send()
                    {
                        Ok(resp) => {
                            let status = resp.status();
                            let text = resp.text().unwrap_or_default();
                            if !status.is_success() {
                                DistributedAssetOutcome {
                                    csv_path,
                                    worker_url: worker_url.clone(),
                                    cache_hit: None,
                                    cache_path: None,
                                    result: None,
                                    error: Some(format!("HTTP {status}: {text}")),
                                }
                            } else {
                                match serde_json::from_str::<WorkerDoubleMaRunResponse>(&text) {
                                    Ok(parsed) => DistributedAssetOutcome {
                                        csv_path: parsed.csv_path,
                                        worker_url: worker_url.clone(),
                                        cache_hit: Some(parsed.cache_hit),
                                        cache_path: Some(parsed.cache_path),
                                        result: Some(parsed.result),
                                        error: None,
                                    },
                                    Err(e) => DistributedAssetOutcome {
                                        csv_path,
                                        worker_url: worker_url.clone(),
                                        cache_hit: None,
                                        cache_path: None,
                                        result: None,
                                        error: Some(format!("failed to parse JSON: {e}")),
                                    },
                                }
                            }
                        }
                        Err(e) => DistributedAssetOutcome {
                            csv_path,
                            worker_url: worker_url.clone(),
                            cache_hit: None,
                            cache_path: None,
                            result: None,
                            error: Some(e.to_string()),
                        },
                    };

                    let _ = tx2.send(outcome);
                }
            }));
        }
        drop(tx);

        let mut results: Vec<DistributedAssetOutcome> = Vec::with_capacity(total);
        let mut completed: usize = 0;
        let mut successes: usize = 0;
        let mut failures: usize = 0;
        let mut cancelled: bool = false;
        let mut cancel_sent: bool = false;

        while completed < total {
            if cancel.load(Ordering::Relaxed) && !cancel_sent {
                cancel_sent = true;
                cancelled = true;
                let _ = app2.emit(
                    "distributed_progress",
                    DistributedProgressPayload {
                        mode: "assets".to_string(),
                        completed,
                        total,
                        successes,
                        failures,
                        last: Some("cancel requested".to_string()),
                    },
                );

                for w in &workers {
                    let _ = client
                        .post(format!("{w}{WORKER_CANCEL_PATH}"))
                        .header(reqwest::header::AUTHORIZATION, authz.clone())
                        .send();
                }
                if let Ok(mut q) = queue.lock() {
                    q.clear();
                }
            }

            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(outcome) => {
                    completed = completed.saturating_add(1);
                    if outcome.error.is_some() {
                        failures = failures.saturating_add(1);
                    } else {
                        successes = successes.saturating_add(1);
                    }
                    let last = Some(outcome.csv_path.clone());
                    results.push(outcome);

                    let _ = app2.emit(
                        "distributed_progress",
                        DistributedProgressPayload {
                            mode: "assets".to_string(),
                            completed,
                            total,
                            successes,
                            failures,
                            last,
                        },
                    );
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if cancel_sent {
                        if threads.iter().all(|t| t.is_finished()) {
                            break;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        for t in threads {
            let _ = t.join();
        }

        Ok(DistributedAssetsRunResponse {
            cancelled,
            total,
            completed,
            successes,
            failures,
            results,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone)]
struct TileSpec {
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
}

#[tauri::command]
pub async fn distributed_run_double_ma_tiled(
    dist_req: DistributedTiledRunRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DistributedTiledRunResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let app2 = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let token = dist_req.auth_token.trim().to_string();
        if token.is_empty() {
            return Err("auth_token is required".to_string());
        }

        let mut workers: Vec<String> = Vec::new();
        for w in dist_req.worker_urls {
            let base = normalize_worker_base_url(&w)?;
            if !workers.contains(&base) {
                workers.push(base);
            }
        }
        if workers.is_empty() {
            return Err("worker_urls is empty".to_string());
        }

        let csv_path = dist_req.csv_path.trim().to_string();
        if csv_path.is_empty() {
            return Err("csv_path is empty".to_string());
        }

        let mut base_req = dist_req.req;
        if base_req.mode != OptimizationMode::Grid {
            return Err("distributed tiling currently requires mode=Grid".to_string());
        }
        if base_req.include_all.unwrap_or(false) {
            return Err("distributed tiling does not support include_all=true".to_string());
        }
        if base_req.export_all_csv_path.is_some() {
            return Err("distributed tiling does not support export_all_csv_path".to_string());
        }
        if base_req.heatmap_bins.unwrap_or(0) > 0 {
            return Err("distributed tiling currently requires heatmap_bins=0".to_string());
        }

        base_req.mode = OptimizationMode::Grid;
        base_req.include_all = Some(false);
        base_req.export_all_csv_path = None;
        base_req.heatmap_bins = Some(0);

        let fast_periods = expand_u16_range(base_req.fast_range)?;
        let slow_periods = expand_u16_range(base_req.slow_range)?;
        if fast_periods.is_empty() || slow_periods.is_empty() {
            return Err("empty period range".to_string());
        }

        let tiles_per_worker = dist_req.tiles_per_worker.unwrap_or(4).clamp(1, 64);
        let target_tiles = workers.len().saturating_mul(tiles_per_worker).max(1);

        let mut f_chunks = (target_tiles as f64).sqrt().ceil() as usize;
        let mut s_chunks = (target_tiles + f_chunks - 1) / f_chunks;
        f_chunks = f_chunks.clamp(1, fast_periods.len());
        s_chunks = s_chunks.clamp(1, slow_periods.len());

        let fast_chunk_len = (fast_periods.len() + f_chunks - 1) / f_chunks;
        let slow_chunk_len = (slow_periods.len() + s_chunks - 1) / s_chunks;

        let mut fast_ranges: Vec<(u32, u32, u32)> = Vec::new();
        for chunk in fast_periods.chunks(fast_chunk_len.max(1)) {
            if chunk.is_empty() {
                continue;
            }
            let start = chunk[0] as u32;
            let end = chunk[chunk.len() - 1] as u32;
            fast_ranges.push((start, end, base_req.fast_range.2));
        }
        let mut slow_ranges: Vec<(u32, u32, u32)> = Vec::new();
        for chunk in slow_periods.chunks(slow_chunk_len.max(1)) {
            if chunk.is_empty() {
                continue;
            }
            let start = chunk[0] as u32;
            let end = chunk[chunk.len() - 1] as u32;
            slow_ranges.push((start, end, base_req.slow_range.2));
        }

        let mut tiles: Vec<TileSpec> = Vec::new();
        for f in &fast_ranges {
            for s in &slow_ranges {
                if f.0 < s.1 {
                    tiles.push(TileSpec {
                        fast_range: *f,
                        slow_range: *s,
                    });
                }
            }
        }
        if tiles.is_empty() {
            return Err("no tiles contain valid fast<slow pairs".to_string());
        }

        let client = http_client()?;
        let authz = authz_header(&token);

        for w in &workers {
            let url = format!("{w}/health");
            let resp = client
                .get(&url)
                .timeout(Duration::from_secs(3))
                .send()
                .map_err(|e| format!("worker {w} health check failed: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("worker {w} health check failed: HTTP {}", resp.status()));
            }
        }

        let tiles_total = tiles.len();
        let queue = Arc::new(Mutex::new(VecDeque::from(tiles)));
        let (tx, rx) = std::sync::mpsc::channel::<Result<WorkerDoubleMaRunResponse, String>>();

        let mut threads = Vec::with_capacity(workers.len());
        for worker_url in workers.clone() {
            let queue2 = queue.clone();
            let tx2 = tx.clone();
            let cancel2 = cancel.clone();
            let client2 = client.clone();
            let authz2 = authz.clone();
            let csv_path2 = csv_path.clone();
            let req2 = base_req.clone();

            threads.push(std::thread::spawn(move || {
                loop {
                    if cancel2.load(Ordering::Relaxed) {
                        break;
                    }
                    let next = {
                        let mut q = match queue2.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        q.pop_front()
                    };
                    let Some(tile) = next else {
                        break;
                    };

                    let mut req_tile = req2.clone();
                    req_tile.fast_range = tile.fast_range;
                    req_tile.slow_range = tile.slow_range;

                    let payload = WorkerDoubleMaRunRequest {
                        csv_path: csv_path2.clone(),
                        req: req_tile,
                    };

                    let url = format!("{worker_url}{WORKER_RUN_DOUBLE_MA_PATH}");
                    let res = client2
                        .post(&url)
                        .header(reqwest::header::AUTHORIZATION, authz2.clone())
                        .json(&payload)
                        .send();

                    let out = match res {
                        Ok(resp) => {
                            let status = resp.status();
                            let text = resp.text().unwrap_or_default();
                            if !status.is_success() {
                                Err(format!("worker {worker_url} HTTP {status}: {text}"))
                            } else {
                                serde_json::from_str::<WorkerDoubleMaRunResponse>(&text)
                                    .map_err(|e| format!("failed to parse JSON: {e}"))
                            }
                        }
                        Err(e) => Err(format!("worker {worker_url} request failed: {e}")),
                    };

                    let _ = tx2.send(out);
                }
            }));
        }
        drop(tx);

        let mut errors: Vec<String> = Vec::new();
        let mut tiles_completed: usize = 0;
        let mut any_gpu: bool = false;
        let objective = base_req.objective;
        let top_k = base_req.top_k.unwrap_or(0);

        let mut best_params: Option<ta_app_core::DoubleMaParamsResolved> = None;
        let mut best_metrics: Option<ta_strategies::double_ma::Metrics> = None;
        let mut best_score: f64 = f64::NEG_INFINITY;

        let mut top_rows: Vec<(ta_app_core::DoubleMaParamsResolved, ta_strategies::double_ma::Metrics, f64)> =
            Vec::new();

        let mut total_combos: usize = 0;
        let mut num_candles: usize = 0;
        let mut runtime_ms: u64 = 0;

        let mut cancel_sent: bool = false;

        while tiles_completed < tiles_total {
            if cancel.load(Ordering::Relaxed) && !cancel_sent {
                cancel_sent = true;
                let _ = app2.emit(
                    "distributed_progress",
                    DistributedProgressPayload {
                        mode: "tiles".to_string(),
                        completed: tiles_completed,
                        total: tiles_total,
                        successes: tiles_completed,
                        failures: errors.len(),
                        last: Some("cancel requested".to_string()),
                    },
                );

                for w in &workers {
                    let _ = client
                        .post(format!("{w}{WORKER_CANCEL_PATH}"))
                        .header(reqwest::header::AUTHORIZATION, authz.clone())
                        .send();
                }
                if let Ok(mut q) = queue.lock() {
                    q.clear();
                }
            }

            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(item) => match item {
                    Ok(res) => {
                        tiles_completed = tiles_completed.saturating_add(1);

                        if matches!(res.result.backend_used, ta_app_core::BackendUsed::Gpu) {
                            any_gpu = true;
                        }
                        total_combos = total_combos.saturating_add(res.result.num_combos);
                        num_candles = num_candles.max(res.result.num_candles);
                        runtime_ms = runtime_ms.saturating_add(res.result.runtime_ms);

                        let m = res.result.best_metrics.clone();
                        let score = match objective {
                            ObjectiveKind::Sharpe => m.sharpe,
                            ObjectiveKind::Sortino => m.sortino,
                            ObjectiveKind::Calmar => m.calmar,
                            ObjectiveKind::Omega => m.omega,
                            ObjectiveKind::Volatility => -m.volatility,
                            ObjectiveKind::Pnl => m.pnl,
                            ObjectiveKind::MaxDrawdown => -m.max_dd,
                        };
                        if score > best_score {
                            best_score = score;
                            best_params = Some(res.result.best_params.clone());
                            best_metrics = Some(m.clone());
                        }

                        if top_k > 0 {
                            for (p, mm) in res.result.top {
                                let sc = match objective {
                                    ObjectiveKind::Sharpe => mm.sharpe,
                                    ObjectiveKind::Sortino => mm.sortino,
                                    ObjectiveKind::Calmar => mm.calmar,
                                    ObjectiveKind::Omega => mm.omega,
                                    ObjectiveKind::Volatility => -mm.volatility,
                                    ObjectiveKind::Pnl => mm.pnl,
                                    ObjectiveKind::MaxDrawdown => -mm.max_dd,
                                };
                                top_rows.push((p, mm, sc));
                            }
                        }

                        let _ = app2.emit(
                            "distributed_progress",
                            DistributedProgressPayload {
                                mode: "tiles".to_string(),
                                completed: tiles_completed,
                                total: tiles_total,
                                successes: tiles_completed,
                                failures: errors.len(),
                                last: None,
                            },
                        );
                    }
                    Err(e) => {
                        errors.push(e);
                        break;
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if cancel_sent {
                        if threads.iter().all(|t| t.is_finished()) {
                            break;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        for t in threads {
            let _ = t.join();
        }

        if cancel.load(Ordering::Relaxed) {
            return Ok(DistributedTiledRunResponse {
                cancelled: true,
                tiles_total,
                tiles_completed,
                errors,
                result: None,
            });
        }

        if !errors.is_empty() {
            return Ok(DistributedTiledRunResponse {
                cancelled: false,
                tiles_total,
                tiles_completed,
                errors,
                result: None,
            });
        }

        if tiles_completed != tiles_total {
            return Ok(DistributedTiledRunResponse {
                cancelled: false,
                tiles_total,
                tiles_completed,
                errors: vec![format!(
                    "tile run incomplete ({tiles_completed}/{tiles_total})"
                )],
                result: None,
            });
        }

        let best_params = best_params.ok_or_else(|| "no best result".to_string())?;
        let best_metrics = best_metrics.ok_or_else(|| "no best result".to_string())?;

        let top: Vec<(ta_app_core::DoubleMaParamsResolved, ta_strategies::double_ma::Metrics)> = if top_k == 0 {
            Vec::new()
        } else {
            top_rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            top_rows
                .into_iter()
                .take(top_k)
                .map(|(p, m, _)| (p, m))
                .collect()
        };

        let (matrix_index, best_col, top_cols) = match ta_app_core::matrix_index::DoubleMaFastLtSlowIndex::new(
            fast_periods,
            slow_periods,
        ) {
            Ok(idx) => {
                let best_col = idx.col_id(best_params.fast_len, best_params.slow_len);
                let mut cols: Vec<u32> = Vec::with_capacity(top.len());
                for (p, _) in &top {
                    match idx.col_id(p.fast_len, p.slow_len) {
                        Some(c) => cols.push(c),
                        None => {
                            cols.clear();
                            break;
                        }
                    }
                }
                let top_cols = if cols.is_empty() && !top.is_empty() {
                    None
                } else {
                    Some(cols)
                };
                (Some(idx), best_col, top_cols)
            }
            Err(_) => (None, None, None),
        };

        let result = ta_app_core::BackendOptimizationResult {
            best_params,
            best_metrics,
            mode_used: ta_app_core::OptimizationModeResolved::Grid,
            backend_used: if any_gpu {
                ta_app_core::BackendUsed::Gpu
            } else {
                ta_app_core::BackendUsed::Cpu
            },
            matrix_index,
            best_col,
            top_cols,
            num_combos: total_combos,
            num_candles,
            runtime_ms,
            top,
            all: None,
            heatmap: None,
        };

        Ok(DistributedTiledRunResponse {
            cancelled: false,
            tiles_total,
            tiles_completed,
            errors: Vec::new(),
            result: Some(result),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
