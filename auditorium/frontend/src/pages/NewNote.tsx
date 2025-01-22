import { useParams, useNavigate } from "react-router";
import { NoteForm } from "../components/NoteForm";
import { useSymphonies } from "@/lib/SymphonyProvider";

export default function NewNote() {
  const navigate = useNavigate();
  const { symphonyId } = useParams();
  const { updateSymphony, trackedSymphonies } = useSymphonies();

  // const handleSubmit = async (data) => {
  //   // This would typically be an API call to create a new note
  //   console.log("Creating new note:", { symphonyId, ...data });
  //   navigate(`/edit-symphony/${symphonyId}`);
  // };

  const handleSubmit = async (data) => {
    const existingSymphony = trackedSymphonies.find(
      (s) => s.name === symphonyId,
    );
    existingSymphony.notes = [...existingSymphony.notes, data];
    console.log(existingSymphony);
    updateSymphony(existingSymphony);
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Create New Note</h2>
      <NoteForm onSubmit={handleSubmit} />
    </div>
  );
}
