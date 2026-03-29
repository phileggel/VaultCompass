import type { UpdateFrequency } from "@/bindings";

export const FREQUENCY_LABELS: Record<UpdateFrequency, string> = {
  Automatic: "Automatic",
  ManualDay: "Daily",
  ManualWeek: "Weekly",
  ManualMonth: "Monthly",
  ManualYear: "Yearly",
};
