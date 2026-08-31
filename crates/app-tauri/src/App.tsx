import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { HighlightedTitle } from "./components/HighlightedTitle";
import type { SearchResultDto } from "./types";
import "./App.css";

function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResultDto[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    invoke<SearchResultDto[]>("search", { query: "" }).then(setResults);

    const win = getCurrentWindow();
    const unlisten = win.listen("tauri://focus", () => {
      inputRef.current?.focus();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function runSearch(nextQuery: string) {
    setQuery(nextQuery);
    const nextResults = await invoke<SearchResultDto[]>("search", {
      query: nextQuery,
    });
    setResults(nextResults);
    setSelectedIndex(0);
  }

  async function launchSelected(index: number) {
    if (results.length === 0) return;
    await invoke("launch", { index });
    setQuery("");
    await getCurrentWindow().hide();
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((i) => (results.length ? (i + 1) % results.length : 0));
        break;
      case "ArrowUp":
      case "Tab":
        e.preventDefault();
        setSelectedIndex((i) =>
          results.length ? (i === 0 ? results.length - 1 : i - 1) : 0,
        );
        break;
      case "Enter":
        e.preventDefault();
        launchSelected(selectedIndex);
        break;
      case "Escape":
        e.preventDefault();
        getCurrentWindow().hide();
        break;
    }
  }

  return (
    <main className="spotlight">
      <input
        ref={inputRef}
        autoFocus
        className="search-bar"
        value={query}
        placeholder="Search apps and settings…"
        onChange={(e) => runSearch(e.currentTarget.value)}
        onKeyDown={onKeyDown}
      />
      {results.length > 0 && (
        <ul className="result-list">
          {results.map((result, index) => (
            <li
              key={index}
              className={
                index === selectedIndex ? "result-item selected" : "result-item"
              }
              onMouseEnter={() => setSelectedIndex(index)}
              onClick={() => launchSelected(index)}
            >
              <span className="result-title">
                <HighlightedTitle
                  text={result.title}
                  matchedIndices={result.matched_indices}
                />
              </span>
              {result.subtitle && (
                <span className="result-subtitle">{result.subtitle}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}

export default App;
