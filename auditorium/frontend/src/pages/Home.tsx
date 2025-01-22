import { Link } from "wouter";
import { Button } from "@/components/ui/button";
import { SymphonyList } from "../components/SymphonyList";

export default function Home() {
  return (
    <div>
      <Link to="/new-symphony">
        <Button className="mb-4">Create New Symphony</Button>
      </Link>
      <SymphonyList />
    </div>
  );
}
