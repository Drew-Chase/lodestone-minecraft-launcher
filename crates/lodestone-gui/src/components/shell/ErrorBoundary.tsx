import React from "react";
import {Button} from "@heroui/react";

interface Props {
    children: React.ReactNode;
}

interface State {
    error: Error | null;
}

/**
 * Per-page error boundary. Catches render-time exceptions in its
 * children and shows a recoverable error card instead of crashing the
 * entire window.
 *
 * Usage:
 *   <ErrorBoundary><SomePage/></ErrorBoundary>
 *
 * Clicking "Reload page" clears the error state so the child tree is
 * re-mounted. Navigation via react-router also resets boundaries
 * automatically because the component key changes.
 */
export default class ErrorBoundary extends React.Component<Props, State> {
    constructor(props: Props) {
        super(props);
        this.state = {error: null};
    }

    static getDerivedStateFromError(error: Error): State {
        return {error};
    }

    componentDidCatch(error: Error, info: React.ErrorInfo) {
        console.error("[ErrorBoundary] Caught:", error, info.componentStack);
    }

    render() {
        if (this.state.error) {
            return (
                <div
                    className="flex-1 flex flex-col items-center justify-center gap-4"
                    style={{padding: 40, background: "var(--bg-0)"}}
                >
                    <div
                        className="border rounded-xl"
                        style={{
                            maxWidth: 480,
                            width: "100%",
                            padding: 28,
                            background: "rgba(255,80,80,0.04)",
                            borderColor: "rgba(255,80,80,0.2)",
                        }}
                    >
                        <div
                            style={{
                                fontSize: 16,
                                fontWeight: 700,
                                color: "#ff5050",
                                marginBottom: 8,
                            }}
                        >
                            Something went wrong
                        </div>
                        <div
                            style={{
                                fontSize: 12,
                                color: "var(--ink-2)",
                                lineHeight: 1.6,
                                marginBottom: 16,
                            }}
                        >
                            An unexpected error occurred while rendering this page.
                            The rest of the application is unaffected.
                        </div>
                        <pre
                            style={{
                                fontSize: 11,
                                fontFamily: "var(--mono)",
                                color: "var(--ink-3)",
                                background: "rgba(0,0,0,0.3)",
                                borderRadius: 8,
                                padding: 12,
                                overflow: "auto",
                                maxHeight: 160,
                                marginBottom: 16,
                                whiteSpace: "pre-wrap",
                                wordBreak: "break-word",
                            }}
                        >
                            {this.state.error.message}
                            {this.state.error.stack && `\n\n${this.state.error.stack}`}
                        </pre>
                        <Button
                            size="sm"
                            variant="bordered"
                            className="border-line text-ink-1"
                            onPress={() => this.setState({error: null})}
                        >
                            Reload page
                        </Button>
                    </div>
                </div>
            );
        }

        return this.props.children;
    }
}
