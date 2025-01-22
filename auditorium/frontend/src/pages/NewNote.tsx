import { useParams, useNavigate } from "react-router";
import { NoteForm } from "../components/NoteForm";
import { CrudNote, CrudSymphony, useSymphonies } from "@/lib/SymphonyProvider";

export default function NewNote() {
  const navigate = useNavigate();
  const { symphonyId } = useParams();
  const { updateSymphony, trackedSymphonies } = useSymphonies();

  const handleSubmit = async (data: CrudNote) => {
    const existingSymphony = trackedSymphonies.find(
      (s) => s.name === symphonyId,
    ) as CrudSymphony;

    existingSymphony.notes = [...existingSymphony.notes, data] as CrudNote[];
    console.log(existingSymphony);
    await updateSymphony(existingSymphony.name, existingSymphony);
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Create New Note</h2>
      <NoteForm onSubmit={handleSubmit} />
    </div>
  );
}
