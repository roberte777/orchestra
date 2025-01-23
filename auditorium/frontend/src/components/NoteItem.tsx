import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { TrackedNote } from "@/lib/SymphonyProvider";
import { EditNote } from "./dialog/edit-note";

export function NoteItem({
  note,
  symphonyId,
}: {
  note: TrackedNote;
  symphonyId: string;
}) {
  const isRunning = note.auditorium_state == "Running";

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
            <div className="inline mr-2">State: {note.state}</div>
            <EditNote note={note} symphonyId={symphonyId}>
              <Button variant="outline" size="sm">
                Edit
              </Button>
            </EditNote>
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
