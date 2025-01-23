import { useSymphonies } from "@/lib/SymphonyProvider";
import { SymphonyItem } from "@/components/symphony-item";

export default function Home() {
  const { trackedSymphonies } = useSymphonies();

  return (
    <div className="space-y-4">
      {trackedSymphonies.map((symphony) => (
        <SymphonyItem key={symphony.name} symphony={symphony} />
      ))}
    </div>
  );
}
