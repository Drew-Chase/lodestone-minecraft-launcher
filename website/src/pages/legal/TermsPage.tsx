import LegalPage from "../../components/legal/LegalPage.tsx";
import terms from "../../content/legal/terms-of-service.md?raw";

export default function TermsPage() {
    return <LegalPage markdown={terms}/>;
}
