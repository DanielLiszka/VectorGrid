import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it } from "vitest";
import MlSignalShellPanel from "./MlSignalShellPanel.vue";

function installMlSignalMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="opt"></section>
    <div id="optVueShellMount"></div>
  `;
}

async function flushPanel(): Promise<void> {
  await nextTick();
  await nextTick();
}

describe("MlSignalShellPanel", () => {
  beforeEach(() => {
    installMlSignalMountShell();
    localStorage.clear();
  });

  it("mounts the dedicated ML signal workspace panel through the shell opt host", async () => {
    const pinia = createPinia();
    setActivePinia(pinia);

    const wrapper = mount(MlSignalShellPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    });
    await flushPanel();

    expect(document.body.textContent || "").toContain(
      "Pattern Recognition + ML signal validation",
    );
    expect(document.querySelector("#mlSignalDatasetExportPath")).not.toBeNull();
    expect(document.querySelector("[data-vue-panel='opt']")?.getAttribute("data-vue-ml-signal")).toBe("1");

    wrapper.unmount();
  });
});
