import { GlobalStyles } from "@mui/material";
import { isBoolean, useSetting } from "@/lib/settings";

/**
 * Switches every transition and animation off when the setting says so. It is one rule for the
 * whole page on purpose, a flag in each component would miss the transitions MUI runs itself.
 */
export default function Motion() {
  const [animations] = useSetting("animations", true, isBoolean);
  if (animations) return null;
  return (
    <GlobalStyles
      styles={{
        "*, *::before, *::after": {
          animationDuration: "0.001ms !important",
          animationIterationCount: "1 !important",
          transitionDuration: "0.001ms !important",
          scrollBehavior: "auto !important",
        },
      }}
    />
  );
}
