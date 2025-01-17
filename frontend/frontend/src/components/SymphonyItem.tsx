import { useState } from "react";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { NoteItem } from "./NoteItem";

export function SymphonyItem({ symphony }) {
  const [isRunning, setIsRunning] = useState(symphony.state === "RUNNING");

  const toggleSymphony = () => {
    // This would typically be an API call
    setIsRunning(!isRunning);
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
