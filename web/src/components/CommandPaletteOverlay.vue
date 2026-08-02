<script setup lang="ts">
import Fuse from "fuse.js";
import type { FuseResult } from "fuse.js";
import { computed, nextTick, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { useAnalysisStore } from "../stores/analysisStore";
import { useCommandActionStore } from "../stores/commandActionStore";
import { useCommandPaletteStore } from "../stores/commandPaletteStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  openTaskCenter,
  useTaskCenterVisibility,
} from "../composables/useTaskCenterVisibility";
import { useUiReadiness } from "../composables/useUiReadiness";
import { vMotionList } from "../directives/motionList";
import {
  createSharedShellShortcutCommands,
  type SharedShellShortcutGroup,
} from "../utils/shellShortcutRegistry";
import { resolveShellResultCommandReadiness } from "../utils/shellCommandReadiness";

interface PaletteCommand {
  id: string;
  group: SharedShellShortcutGroup | "navigation";
  name: string;
  desc: string;
  keys: string[];
  keywords: string[];
  enabled: boolean;
  run: () => void;
}

interface PaletteCommandSearchEntry {
  command: PaletteCommand;
  shortcut: string;
  shortcutLower: string;
  nameWords: string[];
  keywordValues: string[];
  order: number;
}

interface PaletteCommandGroup {
  id: PaletteCommand["group"];
  label: string;
  items: Array<{
    command: PaletteCommand;
    index: number;
  }>;
}

const analysisStore = useAnalysisStore();
const commandActionStore = useCommandActionStore();
const commandPaletteStore = useCommandPaletteStore();
const dataStore = useDataStore();
const optimizeStore = useOptimizeStore();
const settingsStore = useSettingsStore();
const workflowArtifactsStore = useWorkflowArtifactsStore();
const workflowStore = useWorkflowStore();
const taskCenterOpen = useTaskCenterVisibility();

const { canExportCsv, canExportJson } = storeToRefs(analysisStore);
const { canOpenLatestMetaReport, canOpenLatestReport } =
  storeToRefs(commandActionStore);
const { open, query, selectedIndex } = storeToRefs(commandPaletteStore);
const { baseCsvPath, canBrowseCsv, canPolygonOpen, csvPath, dataId, dataTimeframe } = storeToRefs(dataStore);
const { canCancel, runDisabled, validateDisabled } = storeToRefs(optimizeStore);
const { currentRun } = storeToRefs(workflowArtifactsStore);
const { mode, workspace, workflowStep } = storeToRefs(workflowStore);
const { datasetReady } = useUiReadiness();
const commandListId = "commandPaletteListbox";
const paletteInvokerRef = ref<HTMLElement | null>(null);

const resultCommandReadiness = computed(() =>
  resolveShellResultCommandReadiness({
    workspace: workspace.value,
    mode: mode.value,
    datasetReady: datasetReady.value,
    activeDataId: dataId.value,
    activeDatasetPath: baseCsvPath.value || csvPath.value,
    activeTimeframe: dataTimeframe.value,
    currentRun: currentRun.value,
    runDisabled: runDisabled.value,
    validateDisabled: validateDisabled.value,
    canExportCsv: canExportCsv.value,
    canExportJson: canExportJson.value,
  }),
);

const commandGroupMeta: Record<PaletteCommand["group"], string> = {
  actions: "Run control",
  data: "Data sources",
  exports: "Exports and reports",
  layout: "Layout",
  navigation: "Navigation",
};
const commandGroupOrder: PaletteCommand["group"][] = [
  "actions",
  "data",
  "exports",
  "layout",
  "navigation",
];

function setWorkflowStepIfNeeded(step: string): void {
  if (
    String(workspace.value || "").trim() === "workflow" &&
    String(workflowStep.value || "").trim() === step
  )
    return;
  workflowStore.selectWorkflowStep(step);
}

function setWorkspaceIfNeeded(nextWorkspace: string): void {
  if (String(workspace.value || "").trim() === nextWorkspace) return;
  workflowStore.selectWorkspace(nextWorkspace);
}

const inputRef = ref<HTMLInputElement | null>(null);

const sharedShortcutCommands = computed<PaletteCommand[]>(() =>
  createSharedShellShortcutCommands({
    canRun: resultCommandReadiness.value.canRun,
    canCancel: canCancel.value,
    canValidate: resultCommandReadiness.value.canValidate,
    canBrowseCsv: canBrowseCsv.value,
    canPolygonOpen: canPolygonOpen.value,
    canExportCsv: resultCommandReadiness.value.canExportCsv,
    canExportJson: resultCommandReadiness.value.canExportJson,
    canOpenLatestReport: canOpenLatestReport.value,
    canOpenLatestMetaReport: canOpenLatestMetaReport.value,
    controlsPanelVisible: !settingsStore.controlsCollapsed,
    disabledReasons: resultCommandReadiness.value.reasons,
    browseCsv: () => {
      commandActionStore.browseCsv();
    },
    openPolygon: () => {
      commandActionStore.openPolygon();
    },
    toggleControls: () => {
      commandActionStore.toggleControls();
    },
    runOptimization: () => {
      commandActionStore.runOptimization();
    },
    cancelOptimization: () => {
      commandActionStore.cancelOptimization();
    },
    validateOptimization: () => {
      commandActionStore.validateOptimization();
    },
    exportTopCsv: () => {
      commandActionStore.exportTopCsv();
    },
    exportTopJson: () => {
      commandActionStore.exportTopJson();
    },
    openLatestReport: () => {
      commandActionStore.openLatestReport();
    },
    openLatestMetaReport: () => {
      commandActionStore.openLatestMetaReport();
    },
    openTaskCenter: () => {
      openTaskCenter(
        taskCenterOpen,
        paletteInvokerRef.value,
      );
    },
  }),
);

const commands = computed<PaletteCommand[]>(() => {
  return [
    ...sharedShortcutCommands.value,
    {
      id: "go_step_data",
      group: "navigation",
      name: "Go to Workflow Step 1 (Data)",
      desc: "Switch to data-source workflow step.",
      keys: [],
      keywords: ["navigate", "workflow", "data"],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded("data");
      },
    },
    {
      id: "go_step_optimize",
      group: "navigation",
      name: "Go to Workflow Step 2 (Optimize)",
      desc: "Switch to strategy setup workflow step.",
      keys: [],
      keywords: ["navigate", "workflow", "optimize"],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded("optimize");
      },
    },
    {
      id: "go_step_validate",
      group: "navigation",
      name: "Go to Workflow Step 3 (Validate)",
      desc: "Switch to validation workflow step.",
      keys: [],
      keywords: ["navigate", "workflow", "validate"],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded("validate");
      },
    },
    {
      id: "go_step_analyze",
      group: "navigation",
      name: "Go to Workflow Step 4 (Analyze)",
      desc: "Switch to analysis workflow step.",
      keys: [],
      keywords: ["navigate", "workflow", "analyze"],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded("analyze");
      },
    },
    {
      id: "go_step_finalize",
      group: "navigation",
      name: "Go to Workflow Step 5 (Finalizer)",
      desc: "Switch to the post-meta ML confidence and Stage 5 workflow step.",
      keys: [],
      keywords: [
        "navigate",
        "workflow",
        "finalize",
        "stage5",
        "stage4",
        "ml",
        "meta",
      ],
      enabled: true,
      run: () => {
        setWorkflowStepIfNeeded("finalize");
      },
    },
    {
      id: "go_workspace_library",
      group: "navigation",
      name: "Go to Library",
      desc: "Switch to saved run library workspace.",
      keys: [],
      keywords: ["navigate", "library", "history"],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("library");
      },
    },
    {
      id: "go_workspace_aggregate",
      group: "navigation",
      name: "Go to Aggregate",
      desc: "Switch to aggregate/meta analysis workspace.",
      keys: [],
      keywords: ["navigate", "aggregate", "meta"],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("aggregate");
      },
    },
    {
      id: "go_workspace_pro",
      group: "navigation",
      name: "Go to Diagnostics",
      desc: "Switch to pro/debug workspace.",
      keys: [],
      keywords: ["navigate", "pro", "debug"],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("pro");
      },
    },
    {
      id: "go_workspace_portfolio",
      group: "navigation",
      name: "Go to Portfolio",
      desc: "Switch to portfolio feasibility research.",
      keys: [],
      keywords: [
        "navigate",
        "portfolio",
        "feasibility",
        "allocation",
        "replay",
      ],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("portfolio");
      },
    },
    {
      id: "go_workspace_live",
      group: "navigation",
      name: "Go to Live",
      desc: "Switch to the live broker workspace.",
      keys: [],
      keywords: ["navigate", "live", "execution", "broker"],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("live");
      },
    },
    {
      id: "go_workspace_settings",
      group: "navigation",
      name: "Go to Settings",
      desc: "Switch to global settings workspace.",
      keys: [],
      keywords: ["navigate", "settings"],
      enabled: true,
      run: () => {
        setWorkspaceIfNeeded("settings");
      },
    },
  ];
});

const commandSearchIndex = computed<PaletteCommandSearchEntry[]>(() =>
  commands.value.map((command, order) => ({
    command,
    shortcut: command.keys.join(" "),
    shortcutLower: command.keys.join(" ").toLowerCase(),
    nameWords: command.name
      .toLowerCase()
      .split(/\s+/g)
      .filter((word) => word.length > 0),
    keywordValues: command.keywords.map((keyword) => keyword.toLowerCase()),
    order,
  })),
);

const commandFuse = computed(
  () =>
    new Fuse(commandSearchIndex.value, {
      includeScore: true,
      ignoreLocation: true,
      threshold: 0.34,
      keys: [
        { name: "command.name", weight: 0.38 },
        { name: "command.keywords", weight: 0.28 },
        { name: "command.desc", weight: 0.18 },
        { name: "shortcut", weight: 0.1 },
        { name: "command.group", weight: 0.06 },
      ],
    }),
);

function rankingBucket(
  command: PaletteCommandSearchEntry,
  tokens: string[],
): number {
  if (!tokens.length) return 3;

  const exactNamePrefix = tokens.every((token) =>
    command.nameWords.some((word) => word.startsWith(token)),
  );
  if (exactNamePrefix) return 0;

  const keywordPrefix = tokens.every((token) =>
    command.keywordValues.some((keyword) => keyword.startsWith(token)),
  );
  if (keywordPrefix) return 1;

  const shortcutPrefix = tokens.every((token) =>
    command.shortcutLower.includes(token),
  );
  if (shortcutPrefix) return 2;

  return 3;
}

const filteredCommands = computed<PaletteCommand[]>(() => {
  const q = String(query.value || "").trim();
  if (!q) return commands.value;
  const tokens = q
    .toLowerCase()
    .split(/\s+/g)
    .filter((token) => token.length > 0);
  const rankedEntries = commandSearchIndex.value.map((entry) => ({
    entry,
    bucket: rankingBucket(entry, tokens),
  }));

  const prefixMatches = rankedEntries
    .filter((ranked) => ranked.bucket <= 1)
    .sort((a, b) => a.bucket - b.bucket || a.entry.order - b.entry.order)
    .map((ranked) => ranked.entry);
  const seen = new Set(prefixMatches.map((entry) => entry.command.id));
  const bucketByCommandId = new Map(
    rankedEntries.map((ranked) => [ranked.entry.command.id, ranked.bucket]),
  );

  const fuzzyMatches = commandFuse.value
    .search(q)
    .sort(
      (
        a: FuseResult<PaletteCommandSearchEntry>,
        b: FuseResult<PaletteCommandSearchEntry>,
      ) => {
        const bucketDiff =
          (bucketByCommandId.get(a.item.command.id) ?? 3) -
          (bucketByCommandId.get(b.item.command.id) ?? 3);
        if (bucketDiff !== 0) return bucketDiff;
        return (
          (a.score ?? Number.POSITIVE_INFINITY) -
            (b.score ?? Number.POSITIVE_INFINITY) || a.refIndex - b.refIndex
        );
      },
    )
    .map((entry: FuseResult<PaletteCommandSearchEntry>) => entry.item)
    .filter((entry: PaletteCommandSearchEntry) => !seen.has(entry.command.id));

  return [...prefixMatches, ...fuzzyMatches].map((entry) => entry.command);
});

const groupedFilteredCommands = computed<PaletteCommandGroup[]>(() => {
  const groups = new Map<
    PaletteCommand["group"],
    PaletteCommandGroup["items"]
  >();
  const q = String(query.value || "").trim();
  const orderedGroupIds: PaletteCommand["group"][] = [];
  filteredCommands.value.forEach((command, index) => {
    const items = groups.get(command.group) ?? [];
    items.push({ command, index });
    groups.set(command.group, items);
    if (q && !orderedGroupIds.includes(command.group)) {
      orderedGroupIds.push(command.group);
    }
  });
  const visibleGroupOrder = q ? orderedGroupIds : commandGroupOrder;
  return visibleGroupOrder
    .map((groupId) => ({
      id: groupId,
      label: commandGroupMeta[groupId],
      items: groups.get(groupId) ?? [],
    }))
    .filter((group) => group.items.length > 0);
});

const searchSummaryId = "commandPaletteSummary";
const searchSummary = computed(() => {
  const total = commands.value.length;
  const matches = filteredCommands.value.length;
  const q = String(query.value || "").trim();
  if (!q)
    return `${total} commands. Search by action, view, report, or shortcut.`;
  const label = matches === 1 ? "match" : "matches";
  return `${matches} ${label} for “${q}”.`;
});

function commandOptionId(commandId: string): string {
  return `command_option_${String(commandId || "").trim()}`;
}

function commandQaAction(commandId: string): string {
  const normalized = String(commandId || "").trim();
  const overrides: Record<string, string> = {
    browse_csv: "browseCsv",
    cancel_run: "cancelRun",
    export_csv: "exportCsv",
    export_json: "exportJson",
    go_step_analyze: "goStepAnalyze",
    go_step_data: "goStepData",
    go_step_finalize: "goStepFinalize",
    go_step_optimize: "goStepOptimize",
    go_step_validate: "goStepValidate",
    go_workspace_aggregate: "goWorkspaceAggregate",
    go_workspace_library: "goWorkspaceLibrary",
    go_workspace_live: "goWorkspaceLive",
    go_workspace_portfolio: "goWorkspacePortfolio",
    go_workspace_pro: "goWorkspaceDiagnostics",
    go_workspace_settings: "goWorkspaceSettings",
    open_latest_meta_report: "openLatestMetaReport",
    open_latest_report: "openLatestReport",
    open_polygon: "openPolygon",
    open_task_center: "openTaskCenter",
    toggle_controls: "toggleControls",
  };
  return `commandPalette.command.${overrides[normalized] ?? normalized}`;
}

const selectedCommandId = computed(() => {
  const command = filteredCommands.value[selectedIndex.value] ?? null;
  return command ? commandOptionId(command.id) : undefined;
});

watch(filteredCommands, (commandsNow) => {
  commandPaletteStore.clampSelection(commandsNow.length);
});

watch(
  open,
  async (isOpen) => {
    if (!isOpen) return;
    paletteInvokerRef.value =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    await nextTick();
    const input = inputRef.value;
    if (!input) return;
    input.focus();
    input.select();
  },
  { immediate: true },
);

function closePalette(): void {
  commandPaletteStore.closePalette();
}

function onOpenChange(nextOpen: boolean): void {
  if (nextOpen) return;
  closePalette();
}

function runCommand(command: PaletteCommand): void {
  if (!command.enabled) return;
  command.run();
  closePalette();
}

function runSelected(): void {
  const command = filteredCommands.value[selectedIndex.value] ?? null;
  if (!command) return;
  runCommand(command);
}

function onKeydown(event: KeyboardEvent): void {
  if (!open.value) return;
  if (event.key === "Escape") {
    event.preventDefault();
    closePalette();
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    commandPaletteStore.moveSelection(1, filteredCommands.value.length);
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    commandPaletteStore.moveSelection(-1, filteredCommands.value.length);
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    commandPaletteStore.setSelection(0, filteredCommands.value.length);
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    commandPaletteStore.setSelection(
      filteredCommands.value.length - 1,
      filteredCommands.value.length,
    );
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    runSelected();
  }
}
</script>

<template>
  <DialogRoot :open="open" @update:open="onOpenChange">
    <DialogPortal>
      <DialogOverlay class="cmdPaletteBackdrop" />
      <DialogContent
        class="cmdPaletteDialog"
        aria-label="Command palette"
        @keydown.capture="onKeydown"
      >
        <header class="head">
          <DialogTitle class="title">Command Palette</DialogTitle>
          <button
            type="button"
            class="closeBtn"
            aria-label="Close command palette"
            @click="closePalette"
          >
            Close
          </button>
        </header>
        <DialogDescription class="srOnly">
          Search commands, navigate workflow views, and execute supported
          actions.
        </DialogDescription>
        <input
          ref="inputRef"
          type="text"
          class="query"
          role="combobox"
          aria-autocomplete="list"
          aria-haspopup="listbox"
          :aria-expanded="open ? 'true' : 'false'"
          :aria-controls="commandListId"
          :aria-activedescendant="selectedCommandId"
          :aria-describedby="searchSummaryId"
          aria-label="Command search"
          :value="query"
          placeholder="Type a command…"
          @input="
            commandPaletteStore.setQuery(
              String(($event.target as HTMLInputElement).value || ''),
            )
          "
        />
        <div :id="searchSummaryId" class="summary" aria-live="polite">
          {{ searchSummary }}
        </div>
        <div
          :id="commandListId"
          v-motion-list
          class="list overflowShadowY"
          role="listbox"
          aria-label="Available commands"
        >
          <section
            v-for="group in groupedFilteredCommands"
            :key="group.id"
            v-motion-list
            class="group"
            role="group"
            :aria-labelledby="`commandPaletteGroup_${group.id}`"
          >
            <div :id="`commandPaletteGroup_${group.id}`" class="groupLabel">
              {{ group.label }}
            </div>
            <div
              v-for="{ command, index } in group.items"
              :key="command.id"
              :id="commandOptionId(command.id)"
              class="row"
              :data-qa-action="commandQaAction(command.id)"
              :class="{
                selected: selectedIndex === index,
                disabled: !command.enabled,
              }"
              role="option"
              :aria-selected="selectedIndex === index"
              :aria-disabled="!command.enabled"
              @mouseenter="
                commandPaletteStore.setSelection(index, filteredCommands.length)
              "
              @mousedown.prevent
              @click="runCommand(command)"
            >
              <div class="meta">
                <div class="name">{{ command.name }}</div>
                <div class="desc">{{ command.desc }}</div>
              </div>
              <div class="keys">
                <kbd
                  v-for="key in command.keys"
                  :key="`${command.id}_${key}`"
                  >{{ key }}</kbd
                >
              </div>
            </div>
          </section>
          <div v-if="!filteredCommands.length" class="empty">
            No matching commands. Try workflow, report, or shortcut terms.
          </div>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<style scoped>
.cmdPaletteBackdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
  border: 0;
  background: color-mix(in oklab, var(--backdrop) 88%, var(--bg));
  backdrop-filter: blur(12px);
}

.cmdPaletteDialog {
  position: fixed;
  top: 16%;
  left: 50%;
  transform: translateX(-50%);
  z-index: 9999;
  width: min(760px, calc(100vw - 24px));
  border: 1px solid
    color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  border-radius: 14px;
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-overlay) 94%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 94%, var(--surface-working))
  );
  box-shadow: var(--shadow-lg);
  backdrop-filter: blur(12px);
  padding: 0.7rem;
  display: grid;
  gap: 0.6rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.82rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--muted);
}

.closeBtn {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(
    in oklab,
    var(--surface-inline) 88%,
    var(--surface-overlay)
  );
  min-height: 1.5rem;
  padding: 0.25rem 0.5rem;
}

.closeBtn:focus-visible {
  outline: none;
  border-color: var(--focus-border);
  box-shadow: var(--focus-ring);
}

.query {
  width: 100%;
}

.summary {
  font-size: 0.75rem;
  color: var(--muted);
}

.list {
  max-height: min(56vh, 480px);
  overflow: auto;
  display: grid;
  gap: 0.75rem;
}

.group {
  display: grid;
  gap: 0.35rem;
}

.groupLabel {
  font-size: 0.72rem;
  font-weight: 650;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--muted) 86%, var(--ink));
}

.row {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(
    in oklab,
    var(--surface-working) 96%,
    var(--surface-overlay)
  );
  color: inherit;
  cursor: pointer;
  text-align: left;
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.45rem 0.55rem;
}

.row.selected {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 74%, var(--accent));
  background: color-mix(
    in oklab,
    var(--selection-fill) 82%,
    var(--surface-working)
  );
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.row.disabled {
  opacity: 0.58;
  cursor: default;
}

.meta {
  display: grid;
  gap: 0.15rem;
  min-width: 0;
}

.name {
  font-size: 0.83rem;
  font-weight: 600;
}

.desc {
  font-size: 0.75rem;
  color: var(--muted);
}

.keys {
  display: flex;
  flex-wrap: wrap;
  flex: 0 1 auto;
  align-items: center;
  justify-content: flex-end;
  gap: 0.25rem;
  min-width: 0;
}

kbd {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.05rem 0.35rem;
  font-size: 0.7rem;
  background: var(--panel-footer);
}

.empty {
  border: 1px dashed var(--border);
  border-radius: 10px;
  color: var(--muted);
  font-size: 0.8rem;
  padding: 0.6rem;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

@media (max-width: 520px) {
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.35rem;
  }

  .keys {
    justify-content: flex-start;
  }
}
</style>
