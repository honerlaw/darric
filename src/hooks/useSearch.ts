import { useCallback, useEffect, useRef, useState } from "react";
import { searchAll } from "../lib/tauri";
import type { SearchResults } from "../types";

interface UseSearchReturn {
  query: string;
  setQuery: (q: string) => void;
  results: SearchResults | null;
  isSearching: boolean;
  clear: () => void;
}

export function useSearch(): UseSearchReturn {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [isSearching, setIsSearching] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current !== null) clearTimeout(timerRef.current);
    if (query.trim().length === 0) {
      setResults(null);
      setIsSearching(false);
      return;
    }
    setIsSearching(true);
    timerRef.current = setTimeout(() => {
      void searchAll(query).then((r) => {
        setResults(r);
        setIsSearching(false);
      });
    }, 200);
    return () => {
      if (timerRef.current !== null) clearTimeout(timerRef.current);
    };
  }, [query]);

  const clear = useCallback(() => {
    if (timerRef.current !== null) clearTimeout(timerRef.current);
    setQuery("");
    setResults(null);
    setIsSearching(false);
  }, []);

  return { query, setQuery, results, isSearching, clear };
}
