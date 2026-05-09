import { persisted } from "$lib/utils";

export const defaultOutputDir = persisted<string | null>("settings-default-output-dir", null);
