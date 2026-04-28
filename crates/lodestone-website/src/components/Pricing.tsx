import {useLayoutEffect, useRef, useState} from "react";
import {Ic} from "./Icons.tsx";

export default function Pricing() {
    const [yearly, setYearly] = useState(false);
    const toggleRef = useRef<HTMLDivElement>(null);
    const monthRef = useRef<HTMLButtonElement>(null);
    const yearRef = useRef<HTMLButtonElement>(null);
    const [knob, setKnob] = useState({left: 5, width: 0});
    useLayoutEffect(() => {
        const target = yearly ? yearRef.current : monthRef.current;
        if (!target || !toggleRef.current) return;
        const tRect = toggleRef.current.getBoundingClientRect();
        const r = target.getBoundingClientRect();
        setKnob({left: r.left - tRect.left, width: r.width});
    }, [yearly]);

    const tiers = [
        {
            cls: "solo", name: "Solo Sync", monthly: 3, yearly: 29,
            tag: "Sync your instances across every device you own.", cta: "Start Solo Sync",
            feats: [
                {t: "Sync up to 5 of your devices", on: true}, {t: "Unlimited personal instances", on: true},
                {t: "Mods, configs, saves, screenshots", on: true}, {t: "Per-instance world cloud snapshots", on: true},
                {t: "Resume where you left off, anywhere", on: true}, {t: "Share instances with friends", on: false},
            ],
        },
        {
            cls: "featured", name: "Friends", monthly: 6, yearly: 58,
            tag: "Run modpacks with the people you actually play with.", cta: "Get Friends Sync", badge: "MOST POPULAR",
            feats: [
                {t: "Everything in Solo Sync", on: true}, {t: "Share up to 5 instances with friends", on: true},
                {t: "Auto-match versions across the party", on: true}, {t: "Synced world saves for SMP groups", on: true},
                {t: "Live mod-update notifications", on: true}, {t: "Priority server bandwidth", on: true},
            ],
        },
        {
            cls: "unlimited", name: "Unlimited", monthly: 12, yearly: 115,
            tag: "For modpack authors, communities, and overengineers.", cta: "Go Unlimited",
            feats: [
                {t: "Everything in Friends", on: true}, {t: "Unlimited shared instances", on: true},
                {t: "Unlimited devices", on: true}, {t: "Custom CDN — push updates to thousands", on: true},
                {t: "Modpack analytics & crash reports", on: true}, {t: "Priority support · early features", on: true},
            ],
        },
    ];

    const fmtMo = (n: number) => { const v = n / 12; return v % 1 === 0 ? String(v) : v.toFixed(2); };

    return (
        <section id="pricing">
            <div className="container" style={{textAlign: "center"}}>
                <div className="section-eyebrow" style={{justifyContent: "center"}}>PRICING</div>
                <h2 className="section-title" style={{textAlign: "center"}}>The launcher is free. Sync is optional.</h2>
                <p className="section-lede" style={{margin: "0 auto", textAlign: "center"}}>
                    Every feature of Lodestone works locally, forever, at no cost.
                    Subscribe only if you want your instances to follow you between devices — or your whole friend group.
                </p>
                <div ref={toggleRef} className="billing-toggle">
                    <span className="knob" style={{left: knob.left, width: knob.width}}/>
                    <button ref={monthRef} className={!yearly ? "on" : ""} onClick={() => setYearly(false)}>Monthly</button>
                    <button ref={yearRef} className={yearly ? "on" : ""} onClick={() => setYearly(true)}>
                        Yearly <span className="save-pill">SAVE 20%</span>
                    </button>
                </div>
                <div className="pricing-grid">
                    {tiers.map((t) => {
                        const price = yearly ? fmtMo(t.yearly) : t.monthly;
                        return (
                            <div key={t.name} className={`price-card ${t.cls}`} style={{textAlign: "left"}}>
                                {"badge" in t && t.badge && <div className="badge">{t.badge}</div>}
                                <div className="price-name">{t.name}</div>
                                <div className="price-amount">
                                    <span style={{fontSize: 32, color: "var(--ink-3)", fontWeight: 600}}>$</span>
                                    {price}
                                    <span className="per">/ month</span>
                                </div>
                                <div className="price-billed">
                                    {yearly ? (<><s>${t.monthly}/mo</s><span className="save">${t.yearly} billed yearly</span></>) : (<span>billed monthly · cancel anytime</span>)}
                                </div>
                                <div className="price-tagline">{t.tag}</div>
                                <ul className="feats">
                                    {t.feats.map((f, i) => (
                                        <li key={i} className={f.on ? "" : "muted"}>
                                            <span className="ic">{f.on ? <Ic.check size={11}/> : "–"}</span>{f.t}
                                        </li>
                                    ))}
                                </ul>
                                <a className="price-cta" href="#signup"><Ic.arrowR size={13}/>{t.cta}</a>
                            </div>
                        );
                    })}
                </div>
                <div className="price-foot">7-day free trial on every tier · <a href="#sync-faq">how sync works</a></div>
            </div>
        </section>
    );
}
