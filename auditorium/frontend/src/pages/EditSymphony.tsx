import { useParams, useNavigate } from "react-router";
import { SymphonyForm } from "../components/SymphonyForm";

export default function EditSymphony() {
  const navigate = useNavigate();
  const { id } = useParams();

  // This would typically be fetched from an API
  const symphony = {
    id,
    name: `Symphony ${id}`,
  };

  const handleSubmit = async (data) => {
    // This would typically be an API call to update the symphony
    console.log("Updating symphony:", { id, ...data });
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Edit Symphony</h2>
      <SymphonyForm symphony={symphony} onSubmit={handleSubmit} />
    </div>
  );
}
