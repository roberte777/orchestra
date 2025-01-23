import { PropsWithChildren, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { TrackedSymphony, useSymphonies } from "@/lib/SymphonyProvider";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";

const EditSymphonySchema = z.object({
  name: z.string().min(3, {
    message: "Symphony name must be at least 3 characters.",
  }),
});

type EditSymphonyProps = {
  symphony: TrackedSymphony;
};

export function EditSymphony({
  symphony,
  children,
}: PropsWithChildren<EditSymphonyProps>) {
  const { updateSymphony } = useSymphonies();
  const [open, setOpen] = useState(false);
  const form = useForm<z.infer<typeof EditSymphonySchema>>({
    resolver: zodResolver(EditSymphonySchema),
    defaultValues: {
      name: symphony.name,
    },
  });

  const onSubmit = async (data: z.infer<typeof EditSymphonySchema>) => {
    await updateSymphony(symphony.name, { ...data, notes: [] });
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Edit Symphony</DialogTitle>
          <DialogDescription>
            Edit an existing tracked symphony.
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form
            onSubmit={form.handleSubmit(onSubmit)}
            className="w-full space-y-6"
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
                  <FormDescription>The name of the symphony.</FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
            <DialogFooter>
              <Button type="submit">Submit</Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
