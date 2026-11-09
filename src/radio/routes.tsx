import type { RouteObject } from "react-router-dom";
import RadioPage from "./RadioPage";
import TagPage from "./TagPage";
import { RadioTag } from "./paths";

export const radioRoutes: RouteObject[] = [
  { index: true, element: <RadioPage /> },
  { path: RadioTag.path, element: <TagPage /> },
];
