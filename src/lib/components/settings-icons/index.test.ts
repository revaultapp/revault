import { describe, expect, it } from "vitest";

import {
  AppearanceDarkIcon,
  AppearanceLightIcon,
  AppearanceSystemIcon,
  ImageDefaultsIcon,
  LanguageIcon,
  OutputFolderIcon,
  ResetIcon,
  VideoDefaultsIcon,
} from "./index";

describe("Settings vault glyph exports", () => {
  it("exports every Settings-specific icon component", () => {
    expect(AppearanceDarkIcon).toBeDefined();
    expect(AppearanceLightIcon).toBeDefined();
    expect(AppearanceSystemIcon).toBeDefined();
    expect(ImageDefaultsIcon).toBeDefined();
    expect(LanguageIcon).toBeDefined();
    expect(OutputFolderIcon).toBeDefined();
    expect(ResetIcon).toBeDefined();
    expect(VideoDefaultsIcon).toBeDefined();
  });
});
