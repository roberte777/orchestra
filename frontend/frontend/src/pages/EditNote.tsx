import { useParams, useNavigate } from "react-router";
import { NoteForm } from "../components/NoteForm";

export default function EditNote() {
  const navigate = useNavigate();
  const { symphonyId, noteId } = useParams();

  // This would typically be fetched from an API
  const note = {
    id: noteId,
    name: `Note ${noteId}`,
    description: "Sample description",
    host: "localhost",
    command: "echo",
    args: ["Hello", "World"],
    env: { KEY: "VALUE" },
    restart_policy: "always",
  };

  const handleSubmit = async (data) => {
    // This would typically be an API call to update the note
    console.log("Updating note:", { symphonyId, noteId, ...data });
    navigate(`/edit-symphony/${symphonyId}`);
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Edit Note</h2>
      <NoteForm note={note} onSubmit={handleSubmit} />
    </div>
  );
}
