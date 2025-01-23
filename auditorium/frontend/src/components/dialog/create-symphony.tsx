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
import { useSymphonies } from "@/lib/SymphonyProvider";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";

const CreateSymphonySchema = z.object({
  name: z.string().min(3, {
    message: "Symphony name must be at least 3 characters.",
  }),
});

type CreateSymphonyProps = {};

export function CreateSymphony({
  children,
}: PropsWithChildren<CreateSymphonyProps>) {
  const { createSymphony } = useSymphonies();
  const [open, setOpen] = useState(false);
  const form = useForm<z.infer<typeof CreateSymphonySchema>>({
    resolver: zodResolver(CreateSymphonySchema),
    defaultValues: {
      name: "",
    },
  });

  const onSubmit = async (data: z.infer<typeof CreateSymphonySchema>) => {
    await createSymphony({ ...data, notes: [] });
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Create New Symphony</DialogTitle>
          <DialogDescription>
            Create a new tracked symphony to monitor and control.
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
                  <FormDescription>
                    The name of the symphony you want to create.
                  </FormDescription>
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
