import { setup } from "xstate";

export type LiveExecutionTarget =
  | "dry_run"
  | "local_paper"
  | "alpaca_paper"
  | "alpaca_live";

export interface LiveExecutionLifecycleContext {
  symbol: string;
  target: LiveExecutionTarget;
  delayed: boolean;
  recoveryReviewRequired: boolean;
  lastError: string;
}

export type LiveExecutionLifecycleEvent =
  | {
      type: "START_LIVE";
      symbol: string;
      target: LiveExecutionTarget;
      delayed?: boolean;
    }
  | { type: "LIVE_SUBSCRIBED" }
  | { type: "LIVE_STOPPED" }
  | { type: "LIVE_INTERRUPTED"; reason: string }
  | { type: "START_RUNNER" }
  | { type: "RUNNER_STARTED" }
  | { type: "RUNNER_STOPPED" }
  | { type: "RUNNER_FAILED"; reason: string }
  | { type: "BROKER_REVIEW_CONFIRMED" }
  | { type: "EMERGENCY_HALT"; reason?: string }
  | { type: "RESET" };

export type LiveExecutionLifecycleState =
  | "idle"
  | "liveStarting"
  | "liveStreaming"
  | "runnerStarting"
  | "running"
  | "runnerStopped"
  | "recoveryRequired"
  | "halted";

const LIVE_EXECUTION_LIFECYCLE_STATES: ReadonlySet<string> = new Set([
  "idle",
  "liveStarting",
  "liveStreaming",
  "runnerStarting",
  "running",
  "runnerStopped",
  "recoveryRequired",
  "halted",
]);

const defaultContext = (): LiveExecutionLifecycleContext => ({
  symbol: "",
  target: "dry_run",
  delayed: true,
  recoveryReviewRequired: false,
  lastError: "",
});

function normalizeSymbol(value: string): string {
  return String(value || "")
    .trim()
    .replace(/\s+/g, "")
    .toUpperCase();
}

function requiresBrokerRecovery(target: LiveExecutionTarget): boolean {
  return (
    target === "alpaca_paper" ||
    target === "alpaca_live" ||
    target === "local_paper"
  );
}

export const liveExecutionLifecycleMachine = setup({
  types: {
    context: {} as LiveExecutionLifecycleContext,
    events: {} as LiveExecutionLifecycleEvent,
  },
  guards: {
    brokerRecoveryRequired: ({ context }) =>
      requiresBrokerRecovery(context.target),
  },
}).createMachine({
  id: "liveExecutionLifecycle",
  initial: "idle",
  context: defaultContext,
  states: {
    idle: {
      on: {
        START_LIVE: {
          target: "liveStarting",
          actions: ({ context, event }) => {
            context.symbol = normalizeSymbol(event.symbol);
            context.target = event.target;
            context.delayed = event.delayed ?? true;
            context.recoveryReviewRequired = false;
            context.lastError = "";
          },
        },
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = false;
          },
        },
      },
    },
    liveStarting: {
      on: {
        LIVE_SUBSCRIBED: "liveStreaming",
        LIVE_STOPPED: {
          target: "idle",
          actions: ({ context }) => {
            context.recoveryReviewRequired = false;
          },
        },
        LIVE_INTERRUPTED: [
          {
            target: "recoveryRequired",
            guard: "brokerRecoveryRequired",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = true;
            },
          },
          {
            target: "idle",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = false;
            },
          },
        ],
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
      },
    },
    liveStreaming: {
      on: {
        START_RUNNER: "runnerStarting",
        LIVE_STOPPED: "idle",
        LIVE_INTERRUPTED: [
          {
            target: "recoveryRequired",
            guard: "brokerRecoveryRequired",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = true;
            },
          },
          {
            target: "idle",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = false;
            },
          },
        ],
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
      },
    },
    runnerStarting: {
      on: {
        RUNNER_STARTED: "running",
        LIVE_STOPPED: {
          target: "idle",
          actions: ({ context }) => {
            context.recoveryReviewRequired = false;
          },
        },
        RUNNER_FAILED: [
          {
            target: "recoveryRequired",
            guard: "brokerRecoveryRequired",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = true;
            },
          },
          {
            target: "liveStreaming",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = false;
            },
          },
        ],
        RUNNER_STOPPED: "runnerStopped",
        LIVE_INTERRUPTED: {
          target: "recoveryRequired",
          actions: ({ context, event }) => {
            context.lastError = event.reason;
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
      },
    },
    running: {
      on: {
        RUNNER_STOPPED: "runnerStopped",
        LIVE_STOPPED: {
          target: "idle",
          actions: ({ context }) => {
            context.recoveryReviewRequired = false;
          },
        },
        RUNNER_FAILED: [
          {
            target: "recoveryRequired",
            guard: "brokerRecoveryRequired",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = true;
            },
          },
          {
            target: "running",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = false;
            },
          },
        ],
        LIVE_INTERRUPTED: {
          target: "recoveryRequired",
          actions: ({ context, event }) => {
            context.lastError = event.reason;
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
      },
    },
    runnerStopped: {
      on: {
        START_RUNNER: "runnerStarting",
        LIVE_STOPPED: "idle",
        LIVE_INTERRUPTED: [
          {
            target: "recoveryRequired",
            guard: "brokerRecoveryRequired",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = true;
            },
          },
          {
            target: "idle",
            actions: ({ context, event }) => {
              context.lastError = event.reason;
              context.recoveryReviewRequired = false;
            },
          },
        ],
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? "";
            context.recoveryReviewRequired = requiresBrokerRecovery(
              context.target,
            );
          },
        },
      },
    },
    recoveryRequired: {
      on: {
        BROKER_REVIEW_CONFIRMED: {
          target: "idle",
          actions: ({ context }) => {
            context.recoveryReviewRequired = false;
            context.lastError = "";
          },
        },
        EMERGENCY_HALT: {
          target: "halted",
          actions: ({ context, event }) => {
            context.lastError = event.reason ?? context.lastError;
          },
        },
        RESET: {
          target: "idle",
          actions: ({ context }) => {
            Object.assign(context, defaultContext());
          },
        },
      },
    },
    halted: {
      on: {
        BROKER_REVIEW_CONFIRMED: {
          target: "idle",
          actions: ({ context }) => {
            context.recoveryReviewRequired = false;
            context.lastError = "";
          },
        },
        RESET: {
          target: "idle",
          actions: ({ context }) => {
            Object.assign(context, defaultContext());
          },
        },
      },
    },
  },
});

export function liveExecutionLifecycleStateName(
  value: unknown,
): LiveExecutionLifecycleState {
  const normalized = String(value || "").trim();
  return LIVE_EXECUTION_LIFECYCLE_STATES.has(normalized)
    ? (normalized as LiveExecutionLifecycleState)
    : "idle";
}

export function isLiveExecutionBrokerArmed(
  target: LiveExecutionTarget,
): boolean {
  return requiresBrokerRecovery(target);
}
