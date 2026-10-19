import "@testing-library/jest-dom/vitest";

import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach } from "vitest";

afterEach(async () => {
  clearMocks();
  window.location.hash = "";
  // Navigation is asynchronous in jsdom too. Finish this test's hashchange
  // before mounting the next page, whose address generation must be fresh.
  await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); });
});
