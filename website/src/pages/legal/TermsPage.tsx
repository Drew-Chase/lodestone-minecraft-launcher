import LegalPage from "../../components/legal/LegalPage.tsx";

export default function TermsPage() {
    return (
        <LegalPage
            title="Terms of Service"
            paragraphs={[
                "These terms describe how you may use the Lodestone launcher and this website.",
                "Use the launcher and the account features in good faith. Keep your sign-in details to yourself.",
                "The launcher is open source. The code is licensed under the GNU General Public License v3 or later.",
            ]}
        />
    );
}
