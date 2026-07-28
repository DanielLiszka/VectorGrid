import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CompactEvidenceValue from "./CompactEvidenceValue.vue";

const copyTextToDesktopClipboardMock = vi.hoisted(() => vi.fn());

vi.mock("../services/desktopClipboard", () => ({
  copyTextToDesktopClipboard: copyTextToDesktopClipboardMock,
}));

describe("CompactEvidenceValue", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows only the final path segment while preserving and copying the full path", async () => {
    const fullPath =
      "C:\\Users\\researcher\\VectorGrid\\artifacts\\vectorgrid_run_2026-07-24.json";
    copyTextToDesktopClipboardMock.mockResolvedValue({
      ok: true,
      method: "navigator-clipboard",
    });

    const wrapper = mount(CompactEvidenceValue, {
      props: {
        label: "saved run path",
        value: fullPath,
        kind: "path",
      },
      slots: {
        actions: '<button data-test="reveal">Reveal</button>',
      },
    });

    const value = wrapper.get(".compactEvidenceText");
    expect(value.text()).toBe("vectorgrid_run_2026-07-24.json");
    expect(value.attributes("title")).toBe(fullPath);
    expect(value.attributes("aria-label")).toBe(`saved run path: ${fullPath}`);
    expect(wrapper.get('[data-test="reveal"]').text()).toBe("Reveal");

    await wrapper.get('[data-qa-action="evidence.copy"]').trigger("click");

    expect(copyTextToDesktopClipboardMock).toHaveBeenCalledWith(fullPath);
    expect(wrapper.get('[role="status"]').text()).toBe(
      "Copied full saved run path.",
    );
    expect(wrapper.get('[data-qa-action="evidence.copy"]').text()).toBe(
      "Copied",
    );
  });

  it("compacts long hashes and announces clipboard failure without losing the full value", async () => {
    const fullHash =
      "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    copyTextToDesktopClipboardMock.mockResolvedValue({
      ok: false,
      method: "none",
      error: "clipboard unavailable",
    });

    const wrapper = mount(CompactEvidenceValue, {
      props: {
        label: "manifest SHA-256",
        value: fullHash,
        kind: "hash",
      },
    });

    const value = wrapper.get(".compactEvidenceText");
    expect(value.text()).toBe("0123456789…abcdef");
    expect(value.attributes("title")).toBe(fullHash);
    expect(value.attributes("aria-label")).toBe(
      `manifest SHA-256: ${fullHash}`,
    );

    await wrapper.get('[data-qa-action="evidence.copy"]').trigger("click");

    expect(copyTextToDesktopClipboardMock).toHaveBeenCalledWith(fullHash);
    expect(wrapper.get('[role="status"]').text()).toBe(
      "Could not copy full manifest SHA-256.",
    );
    expect(wrapper.get('[data-qa-action="evidence.copy"]').text()).toBe(
      "Retry copy",
    );
  });

  it("handles trailing path separators and unavailable evidence", () => {
    const directory = mount(CompactEvidenceValue, {
      props: {
        label: "dataset path",
        value: "/var/lib/vectorgrid/options/",
        kind: "path",
      },
    });
    const unavailable = mount(CompactEvidenceValue, {
      props: {
        label: "dataset hash",
        value: "—",
        kind: "hash",
      },
    });

    expect(directory.get(".compactEvidenceText").text()).toBe("options");
    expect(unavailable.get(".compactEvidenceText").text()).toBe("—");
    expect(unavailable.get(".compactEvidenceText").attributes("aria-label")).toBe(
      "dataset hash: unavailable",
    );
    expect(
      unavailable.find('[data-qa-action="evidence.copy"]').exists(),
    ).toBe(false);
  });

  it("ignores a stale clipboard completion after the evidence value changes", async () => {
    let finishCopy: ((result: { ok: boolean; method: string }) => void) | undefined;
    copyTextToDesktopClipboardMock.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishCopy = resolve;
        }),
    );
    const wrapper = mount(CompactEvidenceValue, {
      props: {
        label: "saved run path",
        value: "C:\\runs\\old.json",
        kind: "path",
      },
    });

    await wrapper.get('[data-qa-action="evidence.copy"]').trigger("click");
    await wrapper.setProps({ value: "C:\\runs\\new.json" });
    finishCopy?.({ ok: true, method: "navigator-clipboard" });
    await nextTick();

    expect(copyTextToDesktopClipboardMock).toHaveBeenCalledWith(
      "C:\\runs\\old.json",
    );
    expect(wrapper.get(".compactEvidenceText").text()).toBe("new.json");
    expect(wrapper.get('[role="status"]').text()).toBe("");
    expect(wrapper.get('[data-qa-action="evidence.copy"]').text()).toBe("Copy");
  });
});
