import { useSymphonies } from "@/lib/SymphonyProvider";
import { Link } from "wouter";
import { Plus } from "lucide-react";
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "../ui/sidebar";
import { CreateSymphony } from "../dialog/create-symphony";

export const SidebarSymphonyList = () => {
  const { trackedSymphonies } = useSymphonies();

  return (
    <SidebarGroup>
      <SidebarGroupLabel>Symphonies</SidebarGroupLabel>
      <CreateSymphony>
        <SidebarGroupAction>
          <Plus />
          <span className="sr-only">Add Symphony</span>
        </SidebarGroupAction>
      </CreateSymphony>
      <SidebarGroupContent>
        <SidebarMenu>
          {trackedSymphonies.map((symphony) => (
            <SidebarMenuItem key={symphony.name}>
              <SidebarMenuButton asChild>
                <Link to={`/symphony/${symphony.name}`}>
                  <span>{symphony.name}</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
};
