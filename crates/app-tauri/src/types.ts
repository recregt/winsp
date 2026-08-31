export type SearchResultKind =
  | "app"
  | "calculation"
  | "web_search"
  | "system_command";

export interface SearchResultDto {
  title: string;
  subtitle: string | null;
  matched_indices: number[];
  kind: SearchResultKind;
}
