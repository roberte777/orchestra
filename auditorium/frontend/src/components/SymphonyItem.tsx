import { Link } from "wouter";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { NoteItem } from "./NoteItem";
import { TrackedSymphony } from "@/lib/SymphonyProvider";

export function SymphonyItem({ symphony }: { symphony: TrackedSymphony }) {
  const isRunning = symphony.auditorium_state === "Running";

  const toggleSymphony = async () => {
    if (isRunning) {
      await fetch(`/api/v1/symphonies/${symphony.name}/stop`, {
        method: "POST",
      });
    } else {
      // This would typically be an API call
      await fetch("/api/v1/symphonies", {
        method: "POST",
        body: JSON.stringify(symphony),
        headers: {
          "Content-Type": "Application/json",
        },
      });
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex justify-between items-center">
          <span>{symphony.name}</span>
          <div>
            <Button
              onClick={toggleSymphony}
              variant={isRunning ? "destructive" : "default"}
              className="mr-2"
            >
              {isRunning ? "Stop" : "Start"}
            </Button>
            <Link to={`/edit-symphony/${symphony.name}`}>
              <Button variant="outline" className="mr-2">
                Edit
              </Button>
            </Link>
            <Link to={`/new-note/${symphony.name}`}>
              <Button variant="outline">Add Note</Button>
            </Link>
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {symphony.notes.map((note) => (
            <NoteItem key={note.name} note={note} symphonyId={symphony.name} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
