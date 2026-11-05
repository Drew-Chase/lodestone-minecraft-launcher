import type {DashboardNavItem} from "./DashboardLayout.tsx";

// The dashboard sidebar. A page is listed here and routed in main.tsx under the DashboardLayout route.
export const dashboardNav: readonly DashboardNavItem[] = [
    {to: "/dashboard/billing", label: "Billing"},
    {to: "/dashboard/friends", label: "Friends"},
    {to: "/dashboard/devices", label: "Devices"},
];
