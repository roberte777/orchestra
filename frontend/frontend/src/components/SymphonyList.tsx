import { SymphonyItem } from "./SymphonyItem";

// This would typically come from an API or database
const symphonies = [
  {
    id: "1",
    name: "Symphony 1",
    notes: [
      { id: "1", name: "Note 1", state: "RUNNING" },
      { id: "2", name: "Note 2", state: "STOPPED" },
    ],
    state: "RUNNING",
  },
  {
    id: "2",
    name: "Symphony 2",
    notes: [
      { id: "3", name: "Note 3", state: "RUNNING" },
      { id: "4", name: "Note 4", state: "RUNNING" },
    ],
    state: "RUNNING",
  },
];

export function SymphonyList() {
  return (
    <div className="space-y-4">
      {symphonies.map((symphony) => (
        <SymphonyItem key={symphony.id} symphony={symphony} />
      ))}
    </div>
  );
}
