import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useTaskStore } from "../stores/taskStore";
import {
  describeLatestTaskOutcome,
  describeTaskActivity,
  formatOverallProgressLabel,
  taskLifecycleLabel,
} from "../utils/statusPresentation";

export function useTaskLifecycleSummary() {
  const taskStore = useTaskStore();
  const { activeById, activeOrder, centerStatus, history, overallProgress, toasts } =
    storeToRefs(taskStore);

  const activeTasks = computed(() =>
    activeOrder.value
      .map((id) => activeById.value[id])
      .filter((task): task is NonNullable<typeof task> => !!task),
  );
  const activeTaskCount = computed(() => activeTasks.value.length);
  const centerStatusLabel = computed(() =>
    taskLifecycleLabel(centerStatus.value),
  );
  const latestTask = computed(() => history.value[0] ?? null);
  const overallProgressLabel = computed(() =>
    formatOverallProgressLabel(overallProgress.value),
  );
  const latestTaskOutcome = computed(() =>
    describeLatestTaskOutcome(latestTask.value),
  );
  const taskHeadline = computed(() => {
    return describeTaskActivity({
      activeTasks: activeTasks.value,
      latestToast: toasts.value[0] ?? null,
      latestTask: latestTask.value,
    });
  });

  return {
    activeTaskCount,
    activeTasks,
    centerStatusLabel,
    latestTask,
    latestTaskOutcome,
    overallProgressLabel,
    taskHeadline,
  };
}
