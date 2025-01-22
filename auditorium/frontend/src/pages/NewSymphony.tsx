import { useLocation } from "wouter";
import { SymphonyForm } from "../components/SymphonyForm";
import { CrudSymphony, useSymphonies } from "@/lib/SymphonyProvider";

export default function NewSymphony() {
  const [_, navigate] = useLocation();
  const { createSymphony } = useSymphonies();

  const handleSubmit = async (data: CrudSymphony) => {
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
