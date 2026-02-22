import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  invokeLegacyDataApplyTimeframeRuntime,
  invokeLegacyDataBrowseCsvRuntime,
  invokeLegacyDataClosePolygonRuntime,
  invokeLegacyDataBrowseMarketDataDirRuntime,
  invokeLegacyDataClearRuntime,
  invokeLegacyDataLoadRuntime,
  invokeLegacyDataLoadSampleCsvRuntime,
  invokeLegacyDataOpenMarketDataDirRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyDataResetMarketDataDirRuntime,
  invokeLegacyDataSetMarketDataDirRuntime,
  invokeLegacyDistributedCancelRuntime,
  invokeLegacyDistributedFillFromPolygonRuntime,
  invokeLegacyDistributedGenerateTokenRuntime,
  invokeLegacyDistributedRunAssetsRuntime,
  invokeLegacyDistributedRunTilesRuntime,
  invokeLegacyDistributedStartWorkerRuntime,
  invokeLegacyDistributedStopWorkerRuntime,
  invokeLegacyDistributedUseCurrentCsvRuntime,
  invokeLegacyDistributedWorkerStatusRuntime,
  invokeLegacyHistoryApplySavedRunRuntime,
  invokeLegacyHistoryApplySessionRuntime,
  invokeLegacyHistoryClearSessionRuntime,
  invokeLegacyHistoryDeleteSavedRunRuntime,
  invokeLegacyHistoryExportPresetRuntime,
  invokeLegacyHistoryImportPresetRuntime,
  invokeLegacyHistoryLoadSavedRunRuntime,
  invokeLegacyHistoryOpenSavedRunReportRuntime,
  invokeLegacyHistoryRefreshSavedRunsRuntime,
  invokeLegacyHistoryRerunSavedRunRuntime,
  invokeLegacyHistoryRevealSavedRunRuntime,
  invokeLegacyHistorySaveSavedRunTagsRuntime,
  invokeLegacyExecutionClearKeysRuntime,
  invokeLegacyExecutionLiveStartRuntime,
  invokeLegacyExecutionLiveStopRuntime,
  invokeLegacyExecutionRefreshRuntime,
  invokeLegacyExecutionRunOnceRuntime,
  invokeLegacyExecutionRunnerStartRuntime,
  invokeLegacyExecutionRunnerStopRuntime,
  invokeLegacyExecutionTestConnectionRuntime,
  invokeLegacyOptimizeApplyPresetRuntime,
  invokeLegacyOptimizeCancelRuntime,
  invokeLegacyOptimizeGpuParityCheckRuntime,
  invokeLegacyOptimizePresetKernelRuntime,
  invokeLegacyOptimizeRefreshGpuStatusRuntime,
  invokeLegacyOptimizeRunRuntime,
  invokeLegacyOptimizeValidateRuntime,
  invokeLegacyValidationCrossRefreshRuntime,
  invokeLegacyValidationCrossSelectAllRuntime,
  invokeLegacyValidationCrossSelectMatchRuntime,
  invokeLegacyValidationCrossSelectNoneRuntime,
  invokeLegacyValidationExportReportJsonRuntime,
  invokeLegacyValidationExportReportXlsxRuntime,
  invokeLegacyValidationExportShortlistCsvRuntime,
  invokeLegacyValidationExportValidationJsonRuntime,
  invokeLegacyValidationToggleCrossDatasetRuntime,
} from './actions'
import {
  invokeLegacyDataApplyTimeframeRuntime as invokeLegacyDataApplyTimeframeRuntimeImpl,
  invokeLegacyDataBrowseCsvRuntime as invokeLegacyDataBrowseCsvRuntimeImpl,
  invokeLegacyDataClosePolygonRuntime as invokeLegacyDataClosePolygonRuntimeImpl,
  invokeLegacyDataBrowseMarketDataDirRuntime as invokeLegacyDataBrowseMarketDataDirRuntimeImpl,
  invokeLegacyDataClearRuntime as invokeLegacyDataClearRuntimeImpl,
  invokeLegacyDataLoadRuntime as invokeLegacyDataLoadRuntimeImpl,
  invokeLegacyDataLoadSampleCsvRuntime as invokeLegacyDataLoadSampleCsvRuntimeImpl,
  invokeLegacyDataOpenMarketDataDirRuntime as invokeLegacyDataOpenMarketDataDirRuntimeImpl,
  invokeLegacyDataOpenPolygonRuntime as invokeLegacyDataOpenPolygonRuntimeImpl,
  invokeLegacyDataResetMarketDataDirRuntime as invokeLegacyDataResetMarketDataDirRuntimeImpl,
  invokeLegacyDataSetMarketDataDirRuntime as invokeLegacyDataSetMarketDataDirRuntimeImpl,
  invokeLegacyDistributedGenerateTokenRuntime as invokeLegacyDistributedGenerateTokenRuntimeImpl,
  invokeLegacyDistributedStartWorkerRuntime as invokeLegacyDistributedStartWorkerRuntimeImpl,
  invokeLegacyDistributedStopWorkerRuntime as invokeLegacyDistributedStopWorkerRuntimeImpl,
  invokeLegacyDistributedWorkerStatusRuntime as invokeLegacyDistributedWorkerStatusRuntimeImpl,
  invokeLegacyDistributedFillFromPolygonRuntime as invokeLegacyDistributedFillFromPolygonRuntimeImpl,
  invokeLegacyDistributedUseCurrentCsvRuntime as invokeLegacyDistributedUseCurrentCsvRuntimeImpl,
  invokeLegacyDistributedRunAssetsRuntime as invokeLegacyDistributedRunAssetsRuntimeImpl,
  invokeLegacyDistributedRunTilesRuntime as invokeLegacyDistributedRunTilesRuntimeImpl,
  invokeLegacyDistributedCancelRuntime as invokeLegacyDistributedCancelRuntimeImpl,
  invokeLegacyHistoryApplySessionRuntime as invokeLegacyHistoryApplySessionRuntimeImpl,
  invokeLegacyHistoryExportPresetRuntime as invokeLegacyHistoryExportPresetRuntimeImpl,
  invokeLegacyHistoryImportPresetRuntime as invokeLegacyHistoryImportPresetRuntimeImpl,
  invokeLegacyHistoryClearSessionRuntime as invokeLegacyHistoryClearSessionRuntimeImpl,
  invokeLegacyHistorySaveSavedRunTagsRuntime as invokeLegacyHistorySaveSavedRunTagsRuntimeImpl,
  invokeLegacyHistoryRefreshSavedRunsRuntime as invokeLegacyHistoryRefreshSavedRunsRuntimeImpl,
  invokeLegacyHistoryRevealSavedRunRuntime as invokeLegacyHistoryRevealSavedRunRuntimeImpl,
  invokeLegacyHistoryDeleteSavedRunRuntime as invokeLegacyHistoryDeleteSavedRunRuntimeImpl,
  invokeLegacyHistoryLoadSavedRunRuntime as invokeLegacyHistoryLoadSavedRunRuntimeImpl,
  invokeLegacyHistoryApplySavedRunRuntime as invokeLegacyHistoryApplySavedRunRuntimeImpl,
  invokeLegacyHistoryRerunSavedRunRuntime as invokeLegacyHistoryRerunSavedRunRuntimeImpl,
  invokeLegacyHistoryOpenSavedRunReportRuntime as invokeLegacyHistoryOpenSavedRunReportRuntimeImpl,
  invokeLegacyExecutionTestConnectionRuntime as invokeLegacyExecutionTestConnectionRuntimeImpl,
  invokeLegacyExecutionRunOnceRuntime as invokeLegacyExecutionRunOnceRuntimeImpl,
  invokeLegacyExecutionRefreshRuntime as invokeLegacyExecutionRefreshRuntimeImpl,
  invokeLegacyExecutionLiveStartRuntime as invokeLegacyExecutionLiveStartRuntimeImpl,
  invokeLegacyExecutionLiveStopRuntime as invokeLegacyExecutionLiveStopRuntimeImpl,
  invokeLegacyExecutionRunnerStartRuntime as invokeLegacyExecutionRunnerStartRuntimeImpl,
  invokeLegacyExecutionRunnerStopRuntime as invokeLegacyExecutionRunnerStopRuntimeImpl,
  invokeLegacyExecutionClearKeysRuntime as invokeLegacyExecutionClearKeysRuntimeImpl,
  invokeLegacyOptimizeApplyPresetRuntime as invokeLegacyOptimizeApplyPresetRuntimeImpl,
  invokeLegacyOptimizeRunRuntime as invokeLegacyOptimizeRunRuntimeImpl,
  invokeLegacyOptimizeCancelRuntime as invokeLegacyOptimizeCancelRuntimeImpl,
  invokeLegacyOptimizePresetKernelRuntime as invokeLegacyOptimizePresetKernelRuntimeImpl,
  invokeLegacyOptimizeRefreshGpuStatusRuntime as invokeLegacyOptimizeRefreshGpuStatusRuntimeImpl,
  invokeLegacyOptimizeGpuParityCheckRuntime as invokeLegacyOptimizeGpuParityCheckRuntimeImpl,
  invokeLegacyOptimizeValidateRuntime as invokeLegacyOptimizeValidateRuntimeImpl,
  invokeLegacyValidationExportShortlistCsvRuntime as invokeLegacyValidationExportShortlistCsvRuntimeImpl,
  invokeLegacyValidationExportValidationJsonRuntime as invokeLegacyValidationExportValidationJsonRuntimeImpl,
  invokeLegacyValidationExportReportJsonRuntime as invokeLegacyValidationExportReportJsonRuntimeImpl,
  invokeLegacyValidationExportReportXlsxRuntime as invokeLegacyValidationExportReportXlsxRuntimeImpl,
  invokeLegacyValidationCrossRefreshRuntime as invokeLegacyValidationCrossRefreshRuntimeImpl,
  invokeLegacyValidationCrossSelectAllRuntime as invokeLegacyValidationCrossSelectAllRuntimeImpl,
  invokeLegacyValidationCrossSelectNoneRuntime as invokeLegacyValidationCrossSelectNoneRuntimeImpl,
  invokeLegacyValidationCrossSelectMatchRuntime as invokeLegacyValidationCrossSelectMatchRuntimeImpl,
  invokeLegacyValidationToggleCrossDatasetRuntime as invokeLegacyValidationToggleCrossDatasetRuntimeImpl,
} from './app.js'

vi.mock('./app.js', () => ({
  invokeLegacyDataApplyTimeframeRuntime: vi.fn(async () => undefined),
  invokeLegacyDataBrowseCsvRuntime: vi.fn(async () => undefined),
  invokeLegacyDataClosePolygonRuntime: vi.fn(async () => undefined),
  invokeLegacyDataBrowseMarketDataDirRuntime: vi.fn(async () => undefined),
  invokeLegacyDataClearRuntime: vi.fn(async () => undefined),
  invokeLegacyDataLoadRuntime: vi.fn(async () => undefined),
  invokeLegacyDataLoadSampleCsvRuntime: vi.fn(async () => undefined),
  invokeLegacyDataOpenMarketDataDirRuntime: vi.fn(async () => undefined),
  invokeLegacyDataOpenPolygonRuntime: vi.fn(async () => undefined),
  invokeLegacyDataResetMarketDataDirRuntime: vi.fn(async () => undefined),
  invokeLegacyDataSetMarketDataDirRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedGenerateTokenRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedStartWorkerRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedStopWorkerRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedWorkerStatusRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedFillFromPolygonRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedUseCurrentCsvRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedRunAssetsRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedRunTilesRuntime: vi.fn(async () => undefined),
  invokeLegacyDistributedCancelRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryApplySessionRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryExportPresetRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryImportPresetRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryClearSessionRuntime: vi.fn(async () => undefined),
  invokeLegacyHistorySaveSavedRunTagsRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryRefreshSavedRunsRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryRevealSavedRunRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryDeleteSavedRunRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryLoadSavedRunRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryApplySavedRunRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryRerunSavedRunRuntime: vi.fn(async () => undefined),
  invokeLegacyHistoryOpenSavedRunReportRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionTestConnectionRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionRunOnceRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionRefreshRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionLiveStartRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionLiveStopRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionRunnerStartRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionRunnerStopRuntime: vi.fn(async () => undefined),
  invokeLegacyExecutionClearKeysRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeApplyPresetRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeRunRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeCancelRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizePresetKernelRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeRefreshGpuStatusRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeGpuParityCheckRuntime: vi.fn(async () => undefined),
  invokeLegacyOptimizeValidateRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationExportShortlistCsvRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationExportValidationJsonRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationExportReportJsonRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationExportReportXlsxRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationCrossRefreshRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationCrossSelectAllRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationCrossSelectNoneRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationCrossSelectMatchRuntime: vi.fn(async () => undefined),
  invokeLegacyValidationToggleCrossDatasetRuntime: vi.fn(async () => undefined),
}))

const invokeLegacyDataApplyTimeframeRuntimeImplMock = vi.mocked(invokeLegacyDataApplyTimeframeRuntimeImpl)
const invokeLegacyDataBrowseCsvRuntimeImplMock = vi.mocked(invokeLegacyDataBrowseCsvRuntimeImpl)
const invokeLegacyDataClosePolygonRuntimeImplMock = vi.mocked(invokeLegacyDataClosePolygonRuntimeImpl)
const invokeLegacyDataBrowseMarketDataDirRuntimeImplMock = vi.mocked(invokeLegacyDataBrowseMarketDataDirRuntimeImpl)
const invokeLegacyDataClearRuntimeImplMock = vi.mocked(invokeLegacyDataClearRuntimeImpl)
const invokeLegacyDataLoadRuntimeImplMock = vi.mocked(invokeLegacyDataLoadRuntimeImpl)
const invokeLegacyDataLoadSampleCsvRuntimeImplMock = vi.mocked(invokeLegacyDataLoadSampleCsvRuntimeImpl)
const invokeLegacyDataOpenMarketDataDirRuntimeImplMock = vi.mocked(invokeLegacyDataOpenMarketDataDirRuntimeImpl)
const invokeLegacyDataOpenPolygonRuntimeImplMock = vi.mocked(invokeLegacyDataOpenPolygonRuntimeImpl)
const invokeLegacyDataResetMarketDataDirRuntimeImplMock = vi.mocked(invokeLegacyDataResetMarketDataDirRuntimeImpl)
const invokeLegacyDataSetMarketDataDirRuntimeImplMock = vi.mocked(invokeLegacyDataSetMarketDataDirRuntimeImpl)
const invokeLegacyDistributedGenerateTokenRuntimeImplMock = vi.mocked(invokeLegacyDistributedGenerateTokenRuntimeImpl)
const invokeLegacyDistributedStartWorkerRuntimeImplMock = vi.mocked(invokeLegacyDistributedStartWorkerRuntimeImpl)
const invokeLegacyDistributedStopWorkerRuntimeImplMock = vi.mocked(invokeLegacyDistributedStopWorkerRuntimeImpl)
const invokeLegacyDistributedWorkerStatusRuntimeImplMock = vi.mocked(invokeLegacyDistributedWorkerStatusRuntimeImpl)
const invokeLegacyDistributedFillFromPolygonRuntimeImplMock = vi.mocked(invokeLegacyDistributedFillFromPolygonRuntimeImpl)
const invokeLegacyDistributedUseCurrentCsvRuntimeImplMock = vi.mocked(invokeLegacyDistributedUseCurrentCsvRuntimeImpl)
const invokeLegacyDistributedRunAssetsRuntimeImplMock = vi.mocked(invokeLegacyDistributedRunAssetsRuntimeImpl)
const invokeLegacyDistributedRunTilesRuntimeImplMock = vi.mocked(invokeLegacyDistributedRunTilesRuntimeImpl)
const invokeLegacyDistributedCancelRuntimeImplMock = vi.mocked(invokeLegacyDistributedCancelRuntimeImpl)
const invokeLegacyHistoryApplySessionRuntimeImplMock = vi.mocked(invokeLegacyHistoryApplySessionRuntimeImpl)
const invokeLegacyHistoryExportPresetRuntimeImplMock = vi.mocked(invokeLegacyHistoryExportPresetRuntimeImpl)
const invokeLegacyHistoryImportPresetRuntimeImplMock = vi.mocked(invokeLegacyHistoryImportPresetRuntimeImpl)
const invokeLegacyHistoryClearSessionRuntimeImplMock = vi.mocked(invokeLegacyHistoryClearSessionRuntimeImpl)
const invokeLegacyHistorySaveSavedRunTagsRuntimeImplMock = vi.mocked(invokeLegacyHistorySaveSavedRunTagsRuntimeImpl)
const invokeLegacyHistoryRefreshSavedRunsRuntimeImplMock = vi.mocked(invokeLegacyHistoryRefreshSavedRunsRuntimeImpl)
const invokeLegacyHistoryRevealSavedRunRuntimeImplMock = vi.mocked(invokeLegacyHistoryRevealSavedRunRuntimeImpl)
const invokeLegacyHistoryDeleteSavedRunRuntimeImplMock = vi.mocked(invokeLegacyHistoryDeleteSavedRunRuntimeImpl)
const invokeLegacyHistoryLoadSavedRunRuntimeImplMock = vi.mocked(invokeLegacyHistoryLoadSavedRunRuntimeImpl)
const invokeLegacyHistoryApplySavedRunRuntimeImplMock = vi.mocked(invokeLegacyHistoryApplySavedRunRuntimeImpl)
const invokeLegacyHistoryRerunSavedRunRuntimeImplMock = vi.mocked(invokeLegacyHistoryRerunSavedRunRuntimeImpl)
const invokeLegacyHistoryOpenSavedRunReportRuntimeImplMock = vi.mocked(invokeLegacyHistoryOpenSavedRunReportRuntimeImpl)
const invokeLegacyExecutionTestConnectionRuntimeImplMock = vi.mocked(invokeLegacyExecutionTestConnectionRuntimeImpl)
const invokeLegacyExecutionRunOnceRuntimeImplMock = vi.mocked(invokeLegacyExecutionRunOnceRuntimeImpl)
const invokeLegacyExecutionRefreshRuntimeImplMock = vi.mocked(invokeLegacyExecutionRefreshRuntimeImpl)
const invokeLegacyExecutionLiveStartRuntimeImplMock = vi.mocked(invokeLegacyExecutionLiveStartRuntimeImpl)
const invokeLegacyExecutionLiveStopRuntimeImplMock = vi.mocked(invokeLegacyExecutionLiveStopRuntimeImpl)
const invokeLegacyExecutionRunnerStartRuntimeImplMock = vi.mocked(invokeLegacyExecutionRunnerStartRuntimeImpl)
const invokeLegacyExecutionRunnerStopRuntimeImplMock = vi.mocked(invokeLegacyExecutionRunnerStopRuntimeImpl)
const invokeLegacyExecutionClearKeysRuntimeImplMock = vi.mocked(invokeLegacyExecutionClearKeysRuntimeImpl)
const invokeLegacyOptimizeApplyPresetRuntimeImplMock = vi.mocked(invokeLegacyOptimizeApplyPresetRuntimeImpl)
const invokeLegacyOptimizeRunRuntimeImplMock = vi.mocked(invokeLegacyOptimizeRunRuntimeImpl)
const invokeLegacyOptimizeCancelRuntimeImplMock = vi.mocked(invokeLegacyOptimizeCancelRuntimeImpl)
const invokeLegacyOptimizePresetKernelRuntimeImplMock = vi.mocked(invokeLegacyOptimizePresetKernelRuntimeImpl)
const invokeLegacyOptimizeRefreshGpuStatusRuntimeImplMock = vi.mocked(invokeLegacyOptimizeRefreshGpuStatusRuntimeImpl)
const invokeLegacyOptimizeGpuParityCheckRuntimeImplMock = vi.mocked(invokeLegacyOptimizeGpuParityCheckRuntimeImpl)
const invokeLegacyOptimizeValidateRuntimeImplMock = vi.mocked(invokeLegacyOptimizeValidateRuntimeImpl)
const invokeLegacyValidationExportShortlistCsvRuntimeImplMock = vi.mocked(invokeLegacyValidationExportShortlistCsvRuntimeImpl)
const invokeLegacyValidationExportValidationJsonRuntimeImplMock = vi.mocked(invokeLegacyValidationExportValidationJsonRuntimeImpl)
const invokeLegacyValidationExportReportJsonRuntimeImplMock = vi.mocked(invokeLegacyValidationExportReportJsonRuntimeImpl)
const invokeLegacyValidationExportReportXlsxRuntimeImplMock = vi.mocked(invokeLegacyValidationExportReportXlsxRuntimeImpl)
const invokeLegacyValidationCrossRefreshRuntimeImplMock = vi.mocked(invokeLegacyValidationCrossRefreshRuntimeImpl)
const invokeLegacyValidationCrossSelectAllRuntimeImplMock = vi.mocked(invokeLegacyValidationCrossSelectAllRuntimeImpl)
const invokeLegacyValidationCrossSelectNoneRuntimeImplMock = vi.mocked(invokeLegacyValidationCrossSelectNoneRuntimeImpl)
const invokeLegacyValidationCrossSelectMatchRuntimeImplMock = vi.mocked(invokeLegacyValidationCrossSelectMatchRuntimeImpl)
const invokeLegacyValidationToggleCrossDatasetRuntimeImplMock = vi.mocked(invokeLegacyValidationToggleCrossDatasetRuntimeImpl)

describe('legacy action bridge', () => {
  beforeEach(() => {
    invokeLegacyDataApplyTimeframeRuntimeImplMock.mockReset()
    invokeLegacyDataApplyTimeframeRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataBrowseCsvRuntimeImplMock.mockReset()
    invokeLegacyDataBrowseCsvRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataClosePolygonRuntimeImplMock.mockReset()
    invokeLegacyDataClosePolygonRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataBrowseMarketDataDirRuntimeImplMock.mockReset()
    invokeLegacyDataBrowseMarketDataDirRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataClearRuntimeImplMock.mockReset()
    invokeLegacyDataClearRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataLoadRuntimeImplMock.mockReset()
    invokeLegacyDataLoadRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataLoadSampleCsvRuntimeImplMock.mockReset()
    invokeLegacyDataLoadSampleCsvRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataOpenMarketDataDirRuntimeImplMock.mockReset()
    invokeLegacyDataOpenMarketDataDirRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataOpenPolygonRuntimeImplMock.mockReset()
    invokeLegacyDataOpenPolygonRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataResetMarketDataDirRuntimeImplMock.mockReset()
    invokeLegacyDataResetMarketDataDirRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDataSetMarketDataDirRuntimeImplMock.mockReset()
    invokeLegacyDataSetMarketDataDirRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedGenerateTokenRuntimeImplMock.mockReset()
    invokeLegacyDistributedGenerateTokenRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedStartWorkerRuntimeImplMock.mockReset()
    invokeLegacyDistributedStartWorkerRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedStopWorkerRuntimeImplMock.mockReset()
    invokeLegacyDistributedStopWorkerRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedWorkerStatusRuntimeImplMock.mockReset()
    invokeLegacyDistributedWorkerStatusRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedFillFromPolygonRuntimeImplMock.mockReset()
    invokeLegacyDistributedFillFromPolygonRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedUseCurrentCsvRuntimeImplMock.mockReset()
    invokeLegacyDistributedUseCurrentCsvRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedRunAssetsRuntimeImplMock.mockReset()
    invokeLegacyDistributedRunAssetsRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedRunTilesRuntimeImplMock.mockReset()
    invokeLegacyDistributedRunTilesRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyDistributedCancelRuntimeImplMock.mockReset()
    invokeLegacyDistributedCancelRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryApplySessionRuntimeImplMock.mockReset()
    invokeLegacyHistoryApplySessionRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryExportPresetRuntimeImplMock.mockReset()
    invokeLegacyHistoryExportPresetRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryImportPresetRuntimeImplMock.mockReset()
    invokeLegacyHistoryImportPresetRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryClearSessionRuntimeImplMock.mockReset()
    invokeLegacyHistoryClearSessionRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistorySaveSavedRunTagsRuntimeImplMock.mockReset()
    invokeLegacyHistorySaveSavedRunTagsRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryRefreshSavedRunsRuntimeImplMock.mockReset()
    invokeLegacyHistoryRefreshSavedRunsRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryRevealSavedRunRuntimeImplMock.mockReset()
    invokeLegacyHistoryRevealSavedRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryDeleteSavedRunRuntimeImplMock.mockReset()
    invokeLegacyHistoryDeleteSavedRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryLoadSavedRunRuntimeImplMock.mockReset()
    invokeLegacyHistoryLoadSavedRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryApplySavedRunRuntimeImplMock.mockReset()
    invokeLegacyHistoryApplySavedRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryRerunSavedRunRuntimeImplMock.mockReset()
    invokeLegacyHistoryRerunSavedRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyHistoryOpenSavedRunReportRuntimeImplMock.mockReset()
    invokeLegacyHistoryOpenSavedRunReportRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionTestConnectionRuntimeImplMock.mockReset()
    invokeLegacyExecutionTestConnectionRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionRunOnceRuntimeImplMock.mockReset()
    invokeLegacyExecutionRunOnceRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionRefreshRuntimeImplMock.mockReset()
    invokeLegacyExecutionRefreshRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionLiveStartRuntimeImplMock.mockReset()
    invokeLegacyExecutionLiveStartRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionLiveStopRuntimeImplMock.mockReset()
    invokeLegacyExecutionLiveStopRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionRunnerStartRuntimeImplMock.mockReset()
    invokeLegacyExecutionRunnerStartRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionRunnerStopRuntimeImplMock.mockReset()
    invokeLegacyExecutionRunnerStopRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyExecutionClearKeysRuntimeImplMock.mockReset()
    invokeLegacyExecutionClearKeysRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeApplyPresetRuntimeImplMock.mockReset()
    invokeLegacyOptimizeApplyPresetRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeRunRuntimeImplMock.mockReset()
    invokeLegacyOptimizeRunRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeCancelRuntimeImplMock.mockReset()
    invokeLegacyOptimizeCancelRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizePresetKernelRuntimeImplMock.mockReset()
    invokeLegacyOptimizePresetKernelRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeRefreshGpuStatusRuntimeImplMock.mockReset()
    invokeLegacyOptimizeRefreshGpuStatusRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeGpuParityCheckRuntimeImplMock.mockReset()
    invokeLegacyOptimizeGpuParityCheckRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyOptimizeValidateRuntimeImplMock.mockReset()
    invokeLegacyOptimizeValidateRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationExportShortlistCsvRuntimeImplMock.mockReset()
    invokeLegacyValidationExportShortlistCsvRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationExportValidationJsonRuntimeImplMock.mockReset()
    invokeLegacyValidationExportValidationJsonRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationExportReportJsonRuntimeImplMock.mockReset()
    invokeLegacyValidationExportReportJsonRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationExportReportXlsxRuntimeImplMock.mockReset()
    invokeLegacyValidationExportReportXlsxRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationCrossRefreshRuntimeImplMock.mockReset()
    invokeLegacyValidationCrossRefreshRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationCrossSelectAllRuntimeImplMock.mockReset()
    invokeLegacyValidationCrossSelectAllRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationCrossSelectNoneRuntimeImplMock.mockReset()
    invokeLegacyValidationCrossSelectNoneRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationCrossSelectMatchRuntimeImplMock.mockReset()
    invokeLegacyValidationCrossSelectMatchRuntimeImplMock.mockResolvedValue(undefined)
    invokeLegacyValidationToggleCrossDatasetRuntimeImplMock.mockReset()
    invokeLegacyValidationToggleCrossDatasetRuntimeImplMock.mockResolvedValue(undefined)
  })

  it('invokes direct data-apply-timeframe runtime wrapper when available', async () => {
    const ok = await invokeLegacyDataApplyTimeframeRuntime({ timeframe: '5m' })

    expect(ok).toBe(true)
    expect(invokeLegacyDataApplyTimeframeRuntimeImplMock).toHaveBeenCalledWith({ timeframe: '5m' })
  })

  it('invokes direct data-clear runtime wrapper when available', async () => {
    const ok = await invokeLegacyDataClearRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyDataClearRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct data runtime wrappers when available', async () => {
    const ld = await invokeLegacyDataLoadRuntime({ path: 'C:/tmp/aapl.csv' })
    const ls = await invokeLegacyDataLoadSampleCsvRuntime({ index: 1 })
    const bc = await invokeLegacyDataBrowseCsvRuntime()
    const op = await invokeLegacyDataOpenPolygonRuntime()
    const cp = await invokeLegacyDataClosePolygonRuntime()
    const bm = await invokeLegacyDataBrowseMarketDataDirRuntime()
    const sm = await invokeLegacyDataSetMarketDataDirRuntime({ dir: 'C:/md', moveExisting: true })
    const rm = await invokeLegacyDataResetMarketDataDirRuntime()
    const om = await invokeLegacyDataOpenMarketDataDirRuntime()

    expect(ld && ls && bc && op && cp && bm && sm && rm && om).toBe(true)
    expect(invokeLegacyDataLoadRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/tmp/aapl.csv' })
    expect(invokeLegacyDataLoadSampleCsvRuntimeImplMock).toHaveBeenCalledWith({ index: 1 })
    expect(invokeLegacyDataBrowseCsvRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDataOpenPolygonRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDataClosePolygonRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDataBrowseMarketDataDirRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDataSetMarketDataDirRuntimeImplMock).toHaveBeenCalledWith({ dir: 'C:/md', moveExisting: true })
    expect(invokeLegacyDataResetMarketDataDirRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDataOpenMarketDataDirRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-run runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeRunRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeRunRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-apply-preset runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeApplyPresetRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeApplyPresetRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-cancel runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeCancelRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeCancelRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-validate runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeValidateRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeValidateRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-preset-kernel runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizePresetKernelRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizePresetKernelRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-refresh-gpu runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeRefreshGpuStatusRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeRefreshGpuStatusRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct optimize-gpu-parity runtime wrapper when available', async () => {
    const ok = await invokeLegacyOptimizeGpuParityCheckRuntime()

    expect(ok).toBe(true)
    expect(invokeLegacyOptimizeGpuParityCheckRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct execution runtime wrappers when available', async () => {
    const t = await invokeLegacyExecutionTestConnectionRuntime()
    const o = await invokeLegacyExecutionRunOnceRuntime()
    const r = await invokeLegacyExecutionRefreshRuntime()
    const ls = await invokeLegacyExecutionLiveStartRuntime()
    const le = await invokeLegacyExecutionLiveStopRuntime()
    const rs = await invokeLegacyExecutionRunnerStartRuntime()
    const re = await invokeLegacyExecutionRunnerStopRuntime()
    const ck = await invokeLegacyExecutionClearKeysRuntime()

    expect(t && o && r && ls && le && rs && re && ck).toBe(true)
    expect(invokeLegacyExecutionTestConnectionRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionRunOnceRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionRefreshRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionLiveStartRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionLiveStopRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionRunnerStartRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionRunnerStopRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyExecutionClearKeysRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct distributed runtime wrappers when available', async () => {
    const gt = await invokeLegacyDistributedGenerateTokenRuntime()
    const sw = await invokeLegacyDistributedStartWorkerRuntime()
    const xw = await invokeLegacyDistributedStopWorkerRuntime()
    const ws = await invokeLegacyDistributedWorkerStatusRuntime()
    const fp = await invokeLegacyDistributedFillFromPolygonRuntime()
    const uc = await invokeLegacyDistributedUseCurrentCsvRuntime()
    const ra = await invokeLegacyDistributedRunAssetsRuntime()
    const rt = await invokeLegacyDistributedRunTilesRuntime()
    const cc = await invokeLegacyDistributedCancelRuntime()

    expect(gt && sw && xw && ws && fp && uc && ra && rt && cc).toBe(true)
    expect(invokeLegacyDistributedGenerateTokenRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedStartWorkerRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedStopWorkerRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedWorkerStatusRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedFillFromPolygonRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedUseCurrentCsvRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedRunAssetsRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedRunTilesRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyDistributedCancelRuntimeImplMock).toHaveBeenCalledTimes(1)
  })

  it('invokes direct history runtime wrappers when available', async () => {
    const ap = await invokeLegacyHistoryApplySessionRuntime()
    const ep = await invokeLegacyHistoryExportPresetRuntime()
    const ip = await invokeLegacyHistoryImportPresetRuntime()
    const cs = await invokeLegacyHistoryClearSessionRuntime()
    const st = await invokeLegacyHistorySaveSavedRunTagsRuntime({ path: 'C:/runs/a.json', tags: 'a,b' })
    const rf = await invokeLegacyHistoryRefreshSavedRunsRuntime()
    const rv = await invokeLegacyHistoryRevealSavedRunRuntime({ path: 'C:/runs/a.json' })
    const dl = await invokeLegacyHistoryDeleteSavedRunRuntime({ path: 'C:/runs/a.json' })
    const ld = await invokeLegacyHistoryLoadSavedRunRuntime({ path: 'C:/runs/a.json' })
    const as = await invokeLegacyHistoryApplySavedRunRuntime({ path: 'C:/runs/a.json' })
    const rr = await invokeLegacyHistoryRerunSavedRunRuntime({ path: 'C:/runs/a.json' })
    const or = await invokeLegacyHistoryOpenSavedRunReportRuntime({ path: 'C:/runs/a.json' })

    expect(ap && ep && ip && cs && st && rf && rv && dl && ld && as && rr && or).toBe(true)
    expect(invokeLegacyHistoryApplySessionRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyHistoryExportPresetRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyHistoryImportPresetRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyHistoryClearSessionRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyHistorySaveSavedRunTagsRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json', tags: 'a,b' })
    expect(invokeLegacyHistoryRefreshSavedRunsRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyHistoryRevealSavedRunRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
    expect(invokeLegacyHistoryDeleteSavedRunRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
    expect(invokeLegacyHistoryLoadSavedRunRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
    expect(invokeLegacyHistoryApplySavedRunRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
    expect(invokeLegacyHistoryRerunSavedRunRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
    expect(invokeLegacyHistoryOpenSavedRunReportRuntimeImplMock).toHaveBeenCalledWith({ path: 'C:/runs/a.json' })
  })

  it('invokes direct validation runtime wrappers when available', async () => {
    const es = await invokeLegacyValidationExportShortlistCsvRuntime()
    const ev = await invokeLegacyValidationExportValidationJsonRuntime()
    const er = await invokeLegacyValidationExportReportJsonRuntime()
    const ex = await invokeLegacyValidationExportReportXlsxRuntime()
    const cr = await invokeLegacyValidationCrossRefreshRuntime()
    const ca = await invokeLegacyValidationCrossSelectAllRuntime()
    const cn = await invokeLegacyValidationCrossSelectNoneRuntime()
    const cm = await invokeLegacyValidationCrossSelectMatchRuntime()
    const tc = await invokeLegacyValidationToggleCrossDatasetRuntime({ path: 'C:/data/aapl.csv', checked: true })

    expect(es && ev && er && ex && cr && ca && cn && cm && tc).toBe(true)
    expect(invokeLegacyValidationExportShortlistCsvRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationExportValidationJsonRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationExportReportJsonRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationExportReportXlsxRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationCrossRefreshRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationCrossSelectAllRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationCrossSelectNoneRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationCrossSelectMatchRuntimeImplMock).toHaveBeenCalledTimes(1)
    expect(invokeLegacyValidationToggleCrossDatasetRuntimeImplMock).toHaveBeenCalledWith({
      path: 'C:/data/aapl.csv',
      checked: true,
    })
  })
})
