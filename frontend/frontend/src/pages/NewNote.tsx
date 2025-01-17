import { useParams, useNavigate } from "react-router";
import { NoteForm } from "../components/NoteForm";

export default function NewNote() {
  const navigate = useNavigate();
  const { symphonyId } = useParams();

  const handleSubmit = async (data) => {
    // This would typically be an API call to create a new note
    console.log("Creating new note:", { symphonyId, ...data });
    navigate(`/edit-symphony/${symphonyId}`);
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Create New Note</h2>
      <NoteForm onSubmit={handleSubmit} />
    </div>
  );
}
