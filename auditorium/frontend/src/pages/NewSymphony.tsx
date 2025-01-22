import { useNavigate } from "react-router";
import { SymphonyForm } from "../components/SymphonyForm";
import { useSymphonies } from "@/lib/SymphonyProvider";

export default function NewSymphony() {
  const navigate = useNavigate();
  const { createSymphony } = useSymphonies();

  const handleSubmit = async (data) => {
    await createSymphony({ ...data, notes: [] });
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Create New Symphony</h2>
      <SymphonyForm onSubmit={handleSubmit} />
    </div>
  );
}
