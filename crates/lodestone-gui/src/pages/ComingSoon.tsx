import {Card, CardBody} from "@heroui/react";

type ComingSoonProps = {
    name: string;
};

export default function ComingSoon({name}: ComingSoonProps) {
    return (
        <div className="flex items-center justify-center flex-1 bg-bg-0">
            <Card
                style={{
                    background:
                        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                    border: "1px solid var(--line)",
                }}
            >
                <CardBody style={{padding: "32px 40px", textAlign: "center"}}>
                    <h1
                        style={{
                            fontSize: 24,
                            fontWeight: 800,
                            letterSpacing: -0.5,
                            color: "var(--mc-green)",
                            marginBottom: 8,
                            textShadow: "0 0 24px var(--mc-green-glow)",
                        }}
                    >
                        {name}
                    </h1>
                    <p style={{fontSize: 13, color: "var(--ink-2)"}}>Coming soon.</p>
                </CardBody>
            </Card>
        </div>
    );
}
