import { Route, Switch } from "wouter";
import { SymphoniesProvider } from "./lib/SymphonyProvider.tsx";
import { SidebarInset, SidebarProvider } from "./components/ui/sidebar";
import { AppSidebar } from "./components/layout/sidebar";
import { Toaster } from "@/components/ui/toaster";
import Home from "./pages/home";
import Symphony from "./pages/symphony";
import "./index.css";

export default function App() {
  return (
    <SymphoniesProvider>
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <div className="flex flex-1 flex-col gap-4 p-4">
            <Switch>
              <Route path="/" component={Home} />
              <Route path="/symphony/:name">
                {(params) => <Symphony name={params.name} />}
              </Route>
            </Switch>
          </div>
        </SidebarInset>
        <Toaster />
      </SidebarProvider>
    </SymphoniesProvider>
  );
}
