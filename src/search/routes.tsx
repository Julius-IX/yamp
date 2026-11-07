import type { RouteObject } from "react-router-dom";
import { Search } from "./paths";
import SearchResults from "./SearchResults";

/** The results screen. A tab that shows the search bar spreads this into its routes. */
export const searchRoutes: RouteObject[] = [{ path: Search.path, element: <SearchResults /> }];
