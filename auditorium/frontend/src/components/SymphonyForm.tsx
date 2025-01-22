import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  CardFooter,
} from "@/components/ui/card";
import { CrudSymphony, TrackedSymphony } from "@/lib/SymphonyProvider";

export function SymphonyForm({
  symphony,
  onSubmit,
}: {
  symphony?: TrackedSymphony;
  onSubmit: (data: CrudSymphony) => void;
}) {
  const [name, setName] = useState(symphony?.name || "");

  const handleSubmit = (e: React.SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    onSubmit({ ...symphony, name });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{symphony ? "Edit Symphony" : "Create Symphony"}</CardTitle>
      </CardHeader>
      <form onSubmit={handleSubmit}>
        <CardContent>
          <div className="space-y-4">
            <div>
              <label
                htmlFor="name"
                className="block text-sm font-medium text-gray-700"
              >
                Symphony Name
              </label>
              <Input
                type="text"
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
          </div>
        </CardContent>
        <CardFooter>
          <Button type="submit">{symphony ? "Update" : "Create"}</Button>
        </CardFooter>
      </form>
    </Card>
  );
}
