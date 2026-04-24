import {Button} from "@heroui/react";
import {I} from "../shell/icons";

interface PaginationProps {
    hasMore: boolean;
    loading: boolean;
    onLoadMore: () => void;
    resultCount: number;
}

/** Compact inline pagination — sits in the filter/toolbar row. */
export default function Pagination({hasMore, loading, onLoadMore, resultCount}: PaginationProps) {
    return (
        <div className="flex items-center gap-2">
            <span style={{fontSize: 12, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                {resultCount} result{resultCount !== 1 ? "s" : ""}
            </span>
            {hasMore && (
                <Button
                    variant="bordered"
                    size="sm"
                    isLoading={loading}
                    onPress={onLoadMore}
                    startContent={!loading ? <I.plus size={12}/> : undefined}
                    className="text-ink-2 border-line min-w-0 h-7 text-xs"
                >
                    More
                </Button>
            )}
        </div>
    );
}
