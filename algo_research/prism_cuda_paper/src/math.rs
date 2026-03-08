use crate::types::ObjectiveSpec;

pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn orient_maximize(points: &[Vec<f32>], objectives: &[ObjectiveSpec]) -> Vec<Vec<f32>> {
    let mut out = points.to_vec();
    for row in out.iter_mut() {
        for (j, obj) in objectives.iter().enumerate() {
            if !obj.maximize {
                row[j] = -row[j];
            }
        }
    }
    out
}

pub fn dominates_max(a: &[f32], b: &[f32]) -> bool {
    let mut better = false;
    for j in 0..a.len() {
        if a[j] < b[j] {
            return false;
        }
        if a[j] > b[j] {
            better = true;
        }
    }
    better
}

pub fn pareto_front_indices(points: &[Vec<f32>]) -> Vec<usize> {
    let mut keep = vec![true; points.len()];
    for i in 0..points.len() {
        if !keep[i] {
            continue;
        }
        for j in 0..points.len() {
            if i == j || !keep[j] {
                continue;
            }
            if dominates_max(&points[j], &points[i]) {
                keep[i] = false;
                break;
            }
        }
    }
    keep.iter().enumerate().filter_map(|(i, &k)| if k { Some(i) } else { None }).collect()
}

pub fn hypervolume_2d_max(points: &[Vec<f32>], reference: [f32; 2]) -> f32 {
    if points.is_empty() {
        return 0.0;
    }
    let mut front: Vec<[f32; 2]> = pareto_front_indices(points)
        .into_iter()
        .map(|i| [points[i][0], points[i][1]])
        .collect();
    front.sort_by(|a, b| b[0].partial_cmp(&a[0]).unwrap());
    let mut hv = 0.0f32;
    let mut best_y = reference[1];
    for p in front {
        if p[1] > best_y {
            hv += (p[0] - reference[0]).max(0.0) * (p[1] - best_y).max(0.0);
            best_y = p[1];
        }
    }
    hv
}

pub fn hv_gain_2d_max(archive: &[Vec<f32>], point: [f32; 2], reference: [f32; 2]) -> f32 {
    let hv0 = hypervolume_2d_max(archive, reference);
    let mut aug = archive.to_vec();
    aug.push(vec![point[0], point[1]]);
    let hv1 = hypervolume_2d_max(&aug, reference);
    (hv1 - hv0).max(0.0)
}

pub fn mean(xs: &[f32]) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f32>() / xs.len() as f32
}

pub fn stddev(xs: &[f32]) -> f32 {
    if xs.len() <= 1 {
        return 0.0;
    }
    let m = mean(xs);
    let mut acc = 0.0;
    for &x in xs {
        let d = x - m;
        acc += d * d;
    }
    (acc / xs.len() as f32).sqrt()
}

pub fn quantile(mut xs: Vec<f32>, q: f32) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let qq = q.clamp(0.0, 1.0);
    let idx = ((xs.len() - 1) as f32 * qq).round() as usize;
    xs[idx]
}

pub fn normalize_points(points: &[Vec<f32>]) -> Vec<Vec<f32>> {
    if points.is_empty() {
        return Vec::new();
    }
    let d = points[0].len();
    let mut lo = vec![f32::INFINITY; d];
    let mut hi = vec![f32::NEG_INFINITY; d];
    for row in points {
        for j in 0..d {
            lo[j] = lo[j].min(row[j]);
            hi[j] = hi[j].max(row[j]);
        }
    }
    points
        .iter()
        .map(|row| {
            (0..d)
                .map(|j| {
                    let span = (hi[j] - lo[j]).max(1e-8);
                    (row[j] - lo[j]) / span
                })
                .collect::<Vec<f32>>()
        })
        .collect()
}

pub fn crowding_select(points: &[Vec<f32>], limit: usize) -> Vec<usize> {
    if points.len() <= limit {
        return (0..points.len()).collect();
    }
    let x = normalize_points(points);
    let mut chosen: Vec<usize> = vec![0];
    let mut best_sum = f32::NEG_INFINITY;
    for (i, row) in x.iter().enumerate() {
        let s = row.iter().sum::<f32>();
        if s > best_sum {
            best_sum = s;
            chosen[0] = i;
        }
    }
    while chosen.len() < limit {
        let mut best_idx = None;
        let mut best_dist = f32::NEG_INFINITY;
        'outer: for i in 0..x.len() {
            for &c in &chosen {
                if c == i {
                    continue 'outer;
                }
            }
            let mut md = f32::INFINITY;
            for &c in &chosen {
                let mut d2 = 0.0f32;
                for j in 0..x[i].len() {
                    let diff = x[i][j] - x[c][j];
                    d2 += diff * diff;
                }
                md = md.min(d2.sqrt());
            }
            if md > best_dist {
                best_dist = md;
                best_idx = Some(i);
            }
        }
        if let Some(i) = best_idx {
            chosen.push(i);
        } else {
            break;
        }
    }
    chosen
}

pub fn mat_transpose_mul(x: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let n = x.len();
    let p = if n == 0 { 0 } else { x[0].len() };
    let mut out = vec![vec![0.0; p]; p];
    for i in 0..p {
        for j in 0..p {
            let mut acc = 0.0;
            for r in 0..n {
                acc += x[r][i] * x[r][j];
            }
            out[i][j] = acc;
        }
    }
    out
}

pub fn mat_t_mul_y(x: &[Vec<f32>], y: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let n = x.len();
    let p = if n == 0 { 0 } else { x[0].len() };
    let m = if y.is_empty() { 0 } else { y[0].len() };
    let mut out = vec![vec![0.0; m]; p];
    for i in 0..p {
        for j in 0..m {
            let mut acc = 0.0;
            for r in 0..n {
                acc += x[r][i] * y[r][j];
            }
            out[i][j] = acc;
        }
    }
    out
}

pub fn mat_mul_vec(a: &[Vec<f32>], x: &[f32]) -> Vec<f32> {
    a.iter()
        .map(|row| row.iter().zip(x.iter()).map(|(aa, xx)| aa * xx).sum())
        .collect()
}

pub fn mat_mul(a: &[Vec<f32>], b: &[Vec<f32>]) -> Vec<Vec<f32>> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let n = a.len();
    let k = a[0].len();
    let m = b[0].len();
    let mut out = vec![vec![0.0; m]; n];
    for i in 0..n {
        for t in 0..k {
            let ait = a[i][t];
            for j in 0..m {
                out[i][j] += ait * b[t][j];
            }
        }
    }
    out
}

pub fn solve_linear_system(mut a: Vec<Vec<f32>>, mut b: Vec<Vec<f32>>) -> Option<Vec<Vec<f32>>> {
    let n = a.len();
    if n == 0 {
        return Some(Vec::new());
    }
    let m = b[0].len();
    for col in 0..n {
        let mut pivot = col;
        let mut best = a[col][col].abs();
        for r in (col + 1)..n {
            let v = a[r][col].abs();
            if v > best {
                best = v;
                pivot = r;
            }
        }
        if best < 1e-8 {
            return None;
        }
        if pivot != col {
            a.swap(col, pivot);
            b.swap(col, pivot);
        }
        let div = a[col][col];
        for j in col..n {
            a[col][j] /= div;
        }
        for j in 0..m {
            b[col][j] /= div;
        }
        for r in 0..n {
            if r == col {
                continue;
            }
            let factor = a[r][col];
            if factor.abs() < 1e-12 {
                continue;
            }
            for j in col..n {
                a[r][j] -= factor * a[col][j];
            }
            for j in 0..m {
                b[r][j] -= factor * b[col][j];
            }
        }
    }
    Some(b)
}

pub fn ridge_fit(x: &[Vec<f32>], y: &[Vec<f32>], lambda: f32) -> Option<Vec<Vec<f32>>> {
    if x.is_empty() || y.is_empty() {
        return None;
    }
    let mut a = mat_transpose_mul(x);
    for i in 0..a.len() {
        a[i][i] += lambda;
    }
    let rhs = mat_t_mul_y(x, y);
    solve_linear_system(a, rhs)
}
