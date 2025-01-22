import { useParams, useNavigate } from "react-router";
import { useSymphonies } from "@/lib/SymphonyProvider";
import { SymphonyForm } from "../components/SymphonyForm";

export default function EditSymphony() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { trackedSymphonies, updateSymphony } = useSymphonies();

  // Find the existing symphony in our tracked list
  const symphony = trackedSymphonies.find((s) => s.name === id);

  const handleSubmit = async (data) => {
    // Replace the fields in the existing symphony
    const updated = {
      ...symphony,
      ...data,
    };
    await updateSymphony(updated);
    navigate("/");
  };

  if (!symphony) {
    return <div>Symphony not found!</div>;
  }

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Edit Symphony</h2>
      <SymphonyForm symphony={symphony} onSubmit={handleSubmit} />
    </div>
  );
}
