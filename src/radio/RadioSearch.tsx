import { IconButton, InputBase } from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";
import SearchIcon from "@mui/icons-material/Search";
import { radius, subdued, surface } from "@/themes/ui";

/** The field the stations are searched with. Searching follows the typing, the caller decides how soon. */
export default function RadioSearch({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <InputBase
      fullWidth
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => e.key === "Escape" && onChange("")}
      placeholder="Search stations"
      inputProps={{ "aria-label": "Search stations" }}
      startAdornment={<SearchIcon fontSize="small" sx={{ mr: 1, ...subdued }} />}
      endAdornment={
        value ? (
          <IconButton size="small" aria-label="Clear search" edge="end" onClick={() => onChange("")}>
            <CloseIcon fontSize="small" />
          </IconButton>
        ) : null
      }
      sx={{ ...surface, height: 40, px: 2, borderRadius: radius.pill }}
    />
  );
}
