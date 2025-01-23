import { SymphonyItem } from "@/components/symphony-item";
import { useSymphonies } from "@/lib/SymphonyProvider";

export default function Symphony({ name }: { name: string }) {
  const { getSymphony } = useSymphonies();
  const symphony = getSymphony(name);

  if (!symphony) {
    return <></>;
  }

  return <SymphonyItem symphony={symphony} />;
}
