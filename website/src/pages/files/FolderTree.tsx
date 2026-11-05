import type {FolderNode} from "./tree.ts";

interface FolderTreeProps {
    root: FolderNode;
    /** The folder whose files the table shows. "" is every file. */
    selected: string;
    /** Folders the user has collapsed. Folders are open unless they are listed here. */
    collapsed: ReadonlySet<string>;
    onSelect: (path: string) => void;
    onToggle: (path: string) => void;
}

const INDENT_PX = 14;

export default function FolderTree({root, selected, collapsed, onSelect, onToggle}: FolderTreeProps) {
    return (
        <nav className="fs-tree" aria-label="Folders">
            <ul className="fs-tree-list">
                <li>
                    <div className="fs-tree-row">
                        <span className="fs-tree-spacer" aria-hidden="true"/>
                        <button
                            type="button"
                            className="fs-tree-name"
                            aria-current={selected === "" ? "true" : undefined}
                            onClick={() => onSelect("")}
                        >
                            All files
                        </button>
                    </div>
                    <FolderBranch
                        nodes={root.children}
                        depth={1}
                        selected={selected}
                        collapsed={collapsed}
                        onSelect={onSelect}
                        onToggle={onToggle}
                    />
                </li>
            </ul>
        </nav>
    );
}

interface FolderBranchProps extends Omit<FolderTreeProps, "root"> {
    nodes: FolderNode[];
    depth: number;
}

function FolderBranch({nodes, depth, selected, collapsed, onSelect, onToggle}: FolderBranchProps) {
    if (nodes.length === 0) return null;
    return (
        <ul className="fs-tree-list">
            {nodes.map((node) => {
                const hasChildren = node.children.length > 0;
                const open = !collapsed.has(node.path);
                return (
                    <li key={node.path}>
                        <div className="fs-tree-row" style={{paddingLeft: depth * INDENT_PX}}>
                            {hasChildren ? (
                                <button
                                    type="button"
                                    className="fs-tree-toggle"
                                    aria-expanded={open}
                                    aria-label={`${open ? "Collapse" : "Expand"} ${node.name}`}
                                    onClick={() => onToggle(node.path)}
                                >
                                    {open ? "▾" : "▸"}
                                </button>
                            ) : (
                                <span className="fs-tree-spacer" aria-hidden="true"/>
                            )}
                            <button
                                type="button"
                                className="fs-tree-name"
                                aria-current={selected === node.path ? "true" : undefined}
                                onClick={() => onSelect(node.path)}
                            >
                                {node.name}
                            </button>
                        </div>
                        {hasChildren && open && (
                            <FolderBranch
                                nodes={node.children}
                                depth={depth + 1}
                                selected={selected}
                                collapsed={collapsed}
                                onSelect={onSelect}
                                onToggle={onToggle}
                            />
                        )}
                    </li>
                );
            })}
        </ul>
    );
}
