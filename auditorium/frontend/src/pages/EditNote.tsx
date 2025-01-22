import { useLocation, useParams } from "wouter";
import { NoteForm } from "../components/NoteForm";
import { useSymphonies, CrudNote } from "@/lib/SymphonyProvider";

export default function EditNote() {
  const [_, navigate] = useLocation();
  const params = useParams<{
    symphonyId: string;
    noteId: string;
  }>();
  const { trackedSymphonies, updateSymphony } = useSymphonies();
  const note = trackedSymphonies
    .find((s) => s.name === params.symphonyId)
    .notes.find((n) => n.name === params.noteId);

  const handleSubmit = async (data: CrudNote) => {
    const existingSymphony = trackedSymphonies.find(
      (s) => s.name === params.symphonyId,
    );
    existingSymphony.notes = existingSymphony.notes.map((n) => {
      if (n.name === params.noteId) {
        return { ...n, ...data };
      } else {
        return { ...n };
      }
    });
    await updateSymphony(existingSymphony.name, existingSymphony);
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Edit Note</h2>
      <NoteForm note={note} onSubmit={handleSubmit} />
    </div>
  );
}
