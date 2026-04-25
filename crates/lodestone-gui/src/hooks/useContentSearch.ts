import {useCallback, useEffect, useRef, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import type {ContentItem, ContentTypeKey, FilterState, SortKey, SourceKey} from "../types/content";

const DEBOUNCE_MS = 300;
const PER_PAGE = 20;

export interface UseContentSearchParams {
    query: string;
    sort: SortKey;
    source: SourceKey;
    contentType: ContentTypeKey;
    filters: FilterState;
}

export interface UseContentSearchResult {
    results: ContentItem[];
    loading: boolean;
    error: string | null;
    page: number;
    hasMore: boolean;
    sentinelRef: React.RefCallback<HTMLElement>;
}

export default function useContentSearch(
    params: UseContentSearchParams
): UseContentSearchResult {
    const {query, sort, source, contentType, filters} = params;

    const [results, setResults] = useState<ContentItem[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [page, setPage] = useState(0);
    const [hasMore, setHasMore] = useState(true);

    const generation = useRef(0);
    const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

    const loadingRef = useRef(loading);
    loadingRef.current = loading;
    const hasMoreRef = useRef(hasMore);
    hasMoreRef.current = hasMore;
    const pageRef = useRef(page);
    pageRef.current = page;

    const doSearch = useCallback(
        async (searchPage: number, append: boolean) => {
            const gen = ++generation.current;
            setLoading(true);
            setError(null);

            try {
                const items = await invoke<ContentItem[]>("search_content", {
                    query: query || null,
                    sort,
                    platform: source,
                    contentType,
                    page: searchPage,
                    perPage: PER_PAGE,
                    filters,
                });

                if (gen !== generation.current) return;

                if (append) {
                    setResults(prev => [...prev, ...items]);
                } else {
                    setResults(items);
                }
                setHasMore(items.length >= PER_PAGE);
                setPage(searchPage);
            } catch (err) {
                if (gen !== generation.current) return;
                setError(typeof err === "string" ? err : String(err));
                if (!append) setResults([]);
                setHasMore(false);
            } finally {
                if (gen === generation.current) setLoading(false);
            }
        },
        [query, sort, source, contentType, filters]
    );

    // When params change, immediately clear results and show loading,
    // then debounce the actual search.
    useEffect(() => {
        // Immediately clear for a fresh start — the user sees a spinner.
        setResults([]);
        setHasMore(true);
        setLoading(true);
        setPage(0);

        if (debounceTimer.current) clearTimeout(debounceTimer.current);
        debounceTimer.current = setTimeout(() => {
            doSearch(0, false);
        }, query ? DEBOUNCE_MS : 0);
        return () => {
            if (debounceTimer.current) clearTimeout(debounceTimer.current);
        };
    }, [doSearch, query]);

    const loadMore = useCallback(() => {
        if (!loadingRef.current && hasMoreRef.current) {
            doSearch(pageRef.current + 1, true);
        }
    }, [doSearch]);

    // ── Infinite scroll via IntersectionObserver ──
    const observerRef = useRef<IntersectionObserver | null>(null);
    const sentinelNodeRef = useRef<HTMLElement | null>(null);

    const sentinelRef: React.RefCallback<HTMLElement> = useCallback(
        (node: HTMLElement | null) => {
            if (observerRef.current) {
                observerRef.current.disconnect();
                observerRef.current = null;
            }
            sentinelNodeRef.current = node;
            if (!node) return;

            observerRef.current = new IntersectionObserver(
                entries => {
                    if (entries[0]?.isIntersecting) {
                        loadMore();
                    }
                },
                {rootMargin: "0px 0px 200px 0px"},
            );
            observerRef.current.observe(node);
        },
        [loadMore],
    );

    // After a page loads and loading flips to false, re-check if the sentinel
    // is still visible (large screens where content doesn't fill the viewport).
    // If so, trigger another page load immediately.
    useEffect(() => {
        if (loading || !hasMore) return;
        if (!sentinelNodeRef.current) return;

        const rect = sentinelNodeRef.current.getBoundingClientRect();
        if (rect.top < window.innerHeight + 200) {
            loadMore();
        }
    }, [loading, hasMore, loadMore]);

    useEffect(() => {
        return () => {
            observerRef.current?.disconnect();
        };
    }, []);

    return {results, loading, error, page, hasMore, sentinelRef};
}
