import Hero from "../components/Hero.tsx";
import Features from "../components/Features.tsx";
import ShowcaseSplits from "../components/ShowcaseSplits.tsx";
import Steps from "../components/Steps.tsx";
// import Quotes from "../components/Quotes.tsx";
import Pricing from "../components/Pricing.tsx";
import OpenSource from "../components/OpenSource.tsx";
import Footer from "../components/Footer.tsx";

export default function Home() {
    return (
        <>
            <Hero/>
            <Features/>
            <ShowcaseSplits/>
            <Steps/>
            {/*<Quotes/>*/}
            <Pricing/>
            <OpenSource/>
            <Footer/>
        </>
    );
}
