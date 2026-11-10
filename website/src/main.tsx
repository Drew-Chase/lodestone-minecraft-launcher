import React from "react";
import ReactDOM from "react-dom/client";
import {BrowserRouter, Navigate, Route, Routes} from "react-router";

import "./css/index.css";
import "./css/account.css";
import "./css/legal.css";
import "./css/sso.css";
import "./css/billing.css";
import "./css/social.css";
import "./css/brand.css";
import AuthLayout from "./auth/AuthLayout.tsx";
import DashboardLayout from "./components/dashboard/DashboardLayout.tsx";
import {dashboardNav} from "./components/dashboard/nav.ts";
import BillingPage from "./pages/billing/BillingPage.tsx";
import SubscribePage from "./pages/billing/SubscribePage.tsx";
import LinkPage from "./pages/link/LinkPage.tsx";
import FriendsPage from "./pages/friends/FriendsPage.tsx";
import InvitePage from "./pages/friends/InvitePage.tsx";
import DevicesPage from "./pages/devices/DevicesPage.tsx";
import FilesPage from "./pages/files/FilesPage.tsx";
import InstanceBrowserPage from "./pages/files/InstanceBrowserPage.tsx";
import Landing from "./pages/Landing.tsx";
import RegisterPage from "./pages/auth/RegisterPage.tsx";
import LoginPage from "./pages/auth/LoginPage.tsx";
import VerifyEmailPage from "./pages/auth/VerifyEmailPage.tsx";
import ForgotPasswordPage from "./pages/auth/ForgotPasswordPage.tsx";
import ResetPasswordPage from "./pages/auth/ResetPasswordPage.tsx";
import AccountPage from "./pages/auth/AccountPage.tsx";
import TermsPage from "./pages/legal/TermsPage.tsx";
import PrivacyPage from "./pages/legal/PrivacyPage.tsx";
import BrandPage from "./pages/brand/BrandPage.tsx";
import NotFoundPage from "./pages/NotFoundPage.tsx";

ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
        <BrowserRouter>
            <Routes>
                <Route path="/" element={<Landing/>}/>
                <Route element={<AuthLayout/>}>
                    <Route path="/register" element={<RegisterPage/>}/>
                    <Route path="/login" element={<LoginPage/>}/>
                    <Route path="/verify-email" element={<VerifyEmailPage/>}/>
                    <Route path="/forgot-password" element={<ForgotPasswordPage/>}/>
                    <Route path="/reset-password" element={<ResetPasswordPage/>}/>
                    <Route path="/account" element={<AccountPage/>}/>
                    <Route path="/subscribe" element={<SubscribePage/>}/>
                    <Route path="/link" element={<LinkPage/>}/>
                    <Route path="/friends/invite" element={<InvitePage/>}/>
                    <Route path="/dashboard" element={<Navigate to="/dashboard/billing" replace/>}/>
                    <Route element={<DashboardLayout items={dashboardNav}/>}>
                        <Route path="/dashboard/billing" element={<BillingPage/>}/>
                        <Route path="/dashboard/friends" element={<FriendsPage/>}/>
                        <Route path="/dashboard/devices" element={<DevicesPage/>}/>
                        <Route path="/dashboard/files" element={<FilesPage/>}/>
                        <Route path="/dashboard/files/:id" element={<InstanceBrowserPage/>}/>
                    </Route>
                </Route>
                <Route path="/terms" element={<TermsPage/>}/>
                <Route path="/privacy" element={<PrivacyPage/>}/>
                <Route path="/brand" element={<BrandPage/>}/>
                <Route path="*" element={<NotFoundPage/>}/>
            </Routes>
        </BrowserRouter>
    </React.StrictMode>
);
