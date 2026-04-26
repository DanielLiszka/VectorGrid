<template>
  <footer
    id="runFooterShell"
    class="workbenchActionBar"
    role="contentinfo"
    aria-label="Workbench action and status bar"
    data-qa-panel="workbench.actionBar"
  >
    <div id="runFooterVueShellMount"></div>
  </footer>
</template>

<style scoped>
.workbenchActionBar {
  min-width: 0;
  border-top: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--panel-footer) 96%, var(--surface));
  box-shadow: 0 -8px 20px color-mix(in srgb, var(--accent-soft) 10%, transparent);
  padding: 0.36rem 0.72rem;
}

@media (max-width: 760px) {
  .workbenchActionBar {
    padding: 0.42rem 0.5rem;
  }
}
</style>
