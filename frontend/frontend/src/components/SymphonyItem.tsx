import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { NoteItem } from "./NoteItem";

export function SymphonyItem({ symphony }) {
  const isRunning = symphony.state === "Running";

  const toggleSymphony = async () => {
    if (isRunning) {
      const resp = await fetch(`/api/v1/symphonies/${symphony.name}/stop`, {
        method: "POST",
      });
      console.log(resp);
    } else {
      // This would typically be an API call
      const resp = await fetch("/api/v1/symphonies", {
        method: "POST",
        body: JSON.stringify(symphony),
        headers: {
          "Content-Type": "Application/json",
        },
      });
      console.log(resp);
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
            <Link to={`/edit-symphony/${symphony.id}`}>
              <Button variant="outline" className="mr-2">
                Edit
              </Button>
            </Link>
            <Link to={`/new-note/${symphony.id}`}>
              <Button variant="outline">Add Note</Button>
            </Link>
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {symphony.notes.map((note) => (
            <NoteItem key={note.id} note={note} symphonyId={symphony.id} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
