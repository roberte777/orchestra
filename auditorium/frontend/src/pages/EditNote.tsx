import { useNavigate, useParams } from "react-router";
import { NoteForm } from "../components/NoteForm";
import { useSymphonies, CrudNote } from "@/lib/SymphonyProvider";

export default function EditNote() {
  const navigate = useNavigate();
  const { symphonyId, noteId } = useParams();
  const { trackedSymphonies, updateSymphony } = useSymphonies();
  const note = trackedSymphonies
    .find((s) => s.name === symphonyId)
    .notes.find((n) => n.name === noteId);

  const handleSubmit = async (data: CrudNote) => {
    const existingSymphony = trackedSymphonies.find(
      (s) => s.name === symphonyId,
    );
    existingSymphony.notes = existingSymphony.notes.map((n) => {
      if (n.name === noteId) {
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
