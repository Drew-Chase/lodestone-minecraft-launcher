import {Card, CardBody} from "@heroui/react";

type ComingSoonProps = {
    name: string;
};

export default function ComingSoon({name}: ComingSoonProps) {
    return (
        <div className="flex items-center justify-center flex-1 bg-bg-0">
            <Card
                className="border border-line"
                style={{
                    background:
                        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                }}
            >
                <CardBody className="px-10 py-8 text-center">
                    <h1 className="text-2xl font-extrabold tracking-tight text-mc-green mb-2 drop-shadow-[0_0_24px_rgba(34,255,132,0.35)]">
                        {name}
                    </h1>
                    <p className="text-[13px] text-ink-2">Coming soon.</p>
                </CardBody>
            </Card>
        </div>
    );
}
