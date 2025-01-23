import { PropsWithChildren, useState } from "react";
import { CrudNote, TrackedNote, useSymphonies } from "@/lib/SymphonyProvider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";

const NoteFormSchema = z.object({
  name: z.string().min(1, { message: "Name is required." }),
  description: z.string().optional().default(""),
  host: z.string().min(1, { message: "Host is required." }),
  command: z.string().min(1, { message: "Command is required." }),
  args: z.string().optional(),
  env: z.string().optional(),
  restart_policy: z.enum(["Always", "OnFailure", "Never"]).default("Always"),
});

type NoteFormProps = {
  note: TrackedNote;
  symphonyId: string;
};

export const EditNote = ({
  note,
  symphonyId,
  children,
}: PropsWithChildren<NoteFormProps>) => {
  const { updateSymphony } = useSymphonies();
  const [open, setOpen] = useState(false);
  const form = useForm<z.infer<typeof NoteFormSchema>>({
    resolver: zodResolver(NoteFormSchema),
    defaultValues: {
      name: note.name,
      description: note.description,
      host: note.host,
      command: note.command,
      args: note.args.join(" ") || "",
      env: note.env
        ? Object.entries(note.env)
            .map(([key, value]) => `${key}=${value}`)
            .join("\n")
        : "",
      restart_policy: note?.restart_policy || "Always",
    },
  });

  const handleSubmit = async (data: z.infer<typeof NoteFormSchema>) => {
    const submitData: CrudNote = {
      ...data,
      args: data.args?.split(" ").filter((arg) => arg.trim() !== "") || [],
      env: data.env
        ? Object.fromEntries(
            data.env
              .split("\n")
              .filter((line) => line.includes("="))
              .map((line) => line.split("=").map((part) => part.trim())),
          )
        : {},
    };
    console.log(submitData);
    // await updateSymphony(existingSymphony.name, existingSymphony);
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>{note ? "Edit Note" : "Create Note"}</DialogTitle>
          <DialogDescription>
            {note
              ? "Edit the details of your note."
              : "Provide details for the new note."}
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form
            onSubmit={form.handleSubmit(handleSubmit)}
            className="space-y-4"
          >
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Name</FormLabel>
                  <FormControl>
                    <Input {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="description"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Description</FormLabel>
                  <FormControl>
                    <Textarea {...field} rows={3} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="host"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Host</FormLabel>
                  <FormControl>
                    <Input {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="command"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Command</FormLabel>
                  <FormControl>
                    <Input {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="args"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Arguments (space-separated)</FormLabel>
                  <FormControl>
                    <Input {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="env"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>
                    Environment Variables (KEY=VALUE, one per line)
                  </FormLabel>
                  <FormControl>
                    <Textarea {...field} rows={3} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="restart_policy"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Restart Policy</FormLabel>
                  <FormControl>
                    <Select
                      onValueChange={field.onChange}
                      defaultValue={field.value}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Select a verified email to display" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="Always">Always</SelectItem>
                        <SelectItem value="On Failure">On Failure</SelectItem>
                        <SelectItem value="Never">Never</SelectItem>
                      </SelectContent>
                    </Select>
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <DialogFooter>
              <Button type="submit">
                {note ? "Update Note" : "Create Note"}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
};
