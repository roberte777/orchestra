import { Route, Switch } from "wouter";
import { SidebarInset, SidebarProvider } from "./components/ui/sidebar";
import { AppSidebar } from "./components/layout/sidebar";
import { Toaster } from "@/components/ui/toaster";
import Home from "./pages/Home";
import Symphony from "./pages/Symphony";
import "./index.css";

export default function App() {
  return (
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
  );
}
