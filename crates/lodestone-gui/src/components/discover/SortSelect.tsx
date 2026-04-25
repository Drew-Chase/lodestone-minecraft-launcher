import {Dropdown, DropdownTrigger, DropdownMenu, DropdownItem, Button} from "@heroui/react";
import {I} from "../shell/icons";
import type {SortKey} from "../../types/content";

const options: {key: SortKey; label: string}[] = [
    {key: "relevance", label: "Relevance"},
    {key: "downloads", label: "Downloads"},
    {key: "follows", label: "Follows"},
    {key: "updated", label: "Updated"},
    {key: "latest", label: "Newest"},
];

export default function SortSelect({value, onChange}: {value: SortKey; onChange: (v: SortKey) => void}) {
    const current = options.find(o => o.key === value);

    return (
        <Dropdown
            classNames={{
                content: "bg-[#0d1117] border border-line min-w-[140px]",
            }}
        >
            <DropdownTrigger>
                <Button
                    variant="bordered"
                    size="sm"
                    endContent={<I.chevDown size={12}/>}
                    className="border-line bg-[rgba(0,0,0,0.3)] text-[var(--ink-1)] font-semibold text-xs"
                >
                    {current?.label ?? "Sort"}
                </Button>
            </DropdownTrigger>
            <DropdownMenu
                aria-label="Sort by"
                selectionMode="single"
                selectedKeys={new Set([value])}
                onSelectionChange={(keys) => {
                    const selected = [...keys][0] as SortKey;
                    if (selected) onChange(selected);
                }}
                classNames={{
                    list: "gap-0",
                }}
                itemClasses={{
                    base: "text-xs text-[var(--ink-2)] data-[hover=true]:bg-white/[0.04] data-[selectable=true]:focus:bg-white/[0.04] data-[selected=true]:text-[var(--mc-green)]",
                }}
            >
                {options.map(o => (
                    <DropdownItem key={o.key}>{o.label}</DropdownItem>
                ))}
            </DropdownMenu>
        </Dropdown>
    );
}
