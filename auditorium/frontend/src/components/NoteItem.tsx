import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";

export function NoteItem({ note, symphonyId }) {
  const isRunning = note.state == "Running";

  const toggleNote = () => {
    // This would typically be an API call
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex justify-between items-center">
          <span>{note.name}</span>
          <div>
            <Button
              onClick={toggleNote}
              variant={isRunning ? "destructive" : "default"}
              size="sm"
              className="mr-2"
            >
              {isRunning ? "Stop" : "Start"}
            </Button>
            <Link to={`/edit-note/${symphonyId}/${note.name}`}>
              <Button variant="outline" size="sm">
                Edit
              </Button>
            </Link>
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <p>Status: {isRunning ? "Running" : "Stopped"}</p>
        <p>Description: {note.description}</p>
        <p>Host: {note.host}</p>
        <p>Command: {note.command}</p>
      </CardContent>
    </Card>
  );
}
