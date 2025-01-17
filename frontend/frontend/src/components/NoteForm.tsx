import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
  CardFooter,
} from "@/components/ui/card";

export function NoteForm({ note, onSubmit }) {
  const [formData, setFormData] = useState({
    name: note?.name || "",
    description: note?.description || "",
    host: note?.host || "",
    command: note?.command || "",
    args: note?.args?.join(" ") || "",
    env: note?.env
      ? Object.entries(note.env)
          .map(([key, value]) => `${key}=${value}`)
          .join("\n")
      : "",
    restart_policy: note?.restart_policy || "always",
  });

  const handleChange = (e) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = (e) => {
    e.preventDefault();
    const submitData = {
      ...formData,
      args: formData.args.split(" ").filter((arg) => arg.trim() !== ""),
      env: Object.fromEntries(
        formData.env
          .split("\n")
          .filter((line) => line.includes("="))
          .map((line) => line.split("=").map((part) => part.trim())),
      ),
    };
    onSubmit(submitData);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{note ? "Edit Note" : "Create Note"}</CardTitle>
      </CardHeader>
      <form onSubmit={handleSubmit}>
        <CardContent className="space-y-4">
          <div>
            <label
              htmlFor="name"
              className="block text-sm font-medium text-gray-700"
            >
              Name
            </label>
            <Input
              type="text"
              id="name"
              name="name"
              value={formData.name}
              onChange={handleChange}
              required
            />
          </div>
          <div>
            <label
              htmlFor="description"
              className="block text-sm font-medium text-gray-700"
            >
              Description
            </label>
            <Textarea
              id="description"
              name="description"
              value={formData.description}
              onChange={handleChange}
              rows={3}
            />
          </div>
          <div>
            <label
              htmlFor="host"
              className="block text-sm font-medium text-gray-700"
            >
              Host
            </label>
            <Input
              type="text"
              id="host"
              name="host"
              value={formData.host}
              onChange={handleChange}
              required
            />
          </div>
          <div>
            <label
              htmlFor="command"
              className="block text-sm font-medium text-gray-700"
            >
              Command
            </label>
            <Input
              type="text"
              id="command"
              name="command"
              value={formData.command}
              onChange={handleChange}
              required
            />
          </div>
          <div>
            <label
              htmlFor="args"
              className="block text-sm font-medium text-gray-700"
            >
              Arguments (space-separated)
            </label>
            <Input
              type="text"
              id="args"
              name="args"
              value={formData.args}
              onChange={handleChange}
            />
          </div>
          <div>
            <label
              htmlFor="env"
              className="block text-sm font-medium text-gray-700"
            >
              Environment Variables (KEY=VALUE, one per line)
            </label>
            <Textarea
              id="env"
              name="env"
              value={formData.env}
              onChange={handleChange}
              rows={3}
            />
          </div>
          <div>
            <label
              htmlFor="restart_policy"
              className="block text-sm font-medium text-gray-700"
            >
              Restart Policy
            </label>
            <select
              id="restart_policy"
              name="restart_policy"
              value={formData.restart_policy}
              onChange={handleChange}
              className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md"
            >
              <option value="always">Always</option>
              <option value="on-failure">On Failure</option>
              <option value="never">Never</option>
            </select>
          </div>
        </CardContent>
        <CardFooter>
          <Button type="submit">{note ? "Update" : "Create"}</Button>
        </CardFooter>
      </form>
    </Card>
  );
}
