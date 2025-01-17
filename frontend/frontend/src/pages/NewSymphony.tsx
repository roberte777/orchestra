import { useNavigate } from "react-router";
import { SymphonyForm } from "../components/SymphonyForm";

export default function NewSymphony() {
  const navigate = useNavigate();

  const handleSubmit = async (data) => {
    // This would typically be an API call to create a new symphony
    console.log("Creating new symphony:", data);
    navigate("/");
  };

  return (
    <div>
      <h2 className="text-2xl font-bold mb-4">Create New Symphony</h2>
      <SymphonyForm onSubmit={handleSubmit} />
    </div>
  );
}
