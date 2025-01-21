import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  PropsWithChildren,
} from "react";

type SymphonyState = "Running" | "Stopped";
type NoteState = "Pending" | "Running" | "Crashed" | "Completed" | "Terminated";

// Example Note / Symphony shapes
interface Note {
  name: string;
  description: string;
  host: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  restart_policy: string; // Replace with your real RestartPolicy type
  symphony: string; // "Parent" symphony name
  state: NoteState; // Replace with real NoteState type
  desired_state: string; // Replace with real DesiredState type
}

interface Symphony {
  name: string;
  notes: Note[];
  state: SymphonyState;
}

// Define the shape of our context
interface SymphoniesContextValue {
  trackedSymphonies: Symphony[];
  addSymphony: (newSymphony: Symphony) => void;
  removeSymphony: (name: string) => void;
  updateSymphony: (updatedSymphony: Symphony) => void;
}

// Create the context
const SymphoniesContext = createContext<SymphoniesContextValue | undefined>(
  undefined,
);

// Create a custom hook to consume it easily
export const useSymphonies = (): SymphoniesContextValue => {
  const context = useContext(SymphoniesContext);
  if (!context) {
    throw new Error("useSymphonies must be used within a SymphoniesProvider");
  }
  return context;
};

// Hard-coded initial symphonies (for app startup)
const initialSymphonies: Symphony[] = [
  {
    name: "Symphony1",
    state: "Stopped",
    notes: [
      {
        name: "Note1",
        description: "First note",
        host: "me",
        command: "echo",
        args: ["Hello", "World"],
        env: {
          VAR1: "value1",
          VAR2: "value2",
        },
        restart_policy: "Never",
        symphony: "Symphony1",
        state: "Pending",
        desired_state: "Stop",
      },
      {
        name: "Note2",
        description: "Second note",
        host: "me",
        command: "ping",
        args: ["127.0.0.1"],
        env: {
          VAR1: "value1",
        },
        restart_policy: "OnFailure",
        symphony: "Symphony1",
        state: "Pending",
        desired_state: "Stop",
      },
      {
        name: "Note3",
        description: "Second note",
        host: "me",
        command: "ping",
        args: ["127.0.0.1"],
        env: {
          VAR1: "value1",
        },
        restart_policy: "OnFailure",
        symphony: "Symphony1",
        state: "Pending",
        desired_state: "Stop",
      },
    ],
  },
  {
    name: "MyHardcodedSymphony",
    state: "Stopped",
    notes: [
      {
        name: "HardcodedNote1",
        description: "Example note",
        host: "localhost",
        command: "echo",
        args: ["Hello", "World"],
        env: {},
        restart_policy: "OnFailure",
        symphony: "MyHardcodedSymphony",
        state: "Running",
        desired_state: "Stop",
      },
    ],
  },
  // Add more if desired...
];

// Provider component
export const SymphoniesProvider: React.FC<PropsWithChildren> = ({
  children,
}) => {
  // "trackedSymphonies" includes our initial/hard-coded ones + any user-added
  const [trackedSymphonies, setTrackedSymphonies] =
    useState<Symphony[]>(initialSymphonies);

  // 1. Poll the server for symphonies every X seconds
  // 2. If the server returns a symphony matching one in trackedSymphonies by name,
  //    update the local trackedSymphonies data for that item.
  useEffect(() => {
    async function fetchServerSymphonies() {
      try {
        const resp = await fetch("/api/v1/symphonies");
        if (!resp.ok) {
          throw new Error("Failed to fetch symphonies");
        }
        const serverSymphonies: Symphony[] = await resp.json();

        // Update any symphony in trackedSymphonies if the server returns
        // a matching name. We do NOT add new ones automatically, because
        // "tracked" means the user explicitly wants to track them.
        setTrackedSymphonies((prev) => {
          return prev.map((localSym) => {
            // Check if serverSymphonies has a matching name
            const match = serverSymphonies.find(
              (srvSym) => srvSym.name === localSym.name,
            );
            if (match) {
              localSym.state = "Running";
              localSym.notes = localSym.notes.map((note) => {
                const noteMatch = match.notes.find(
                  (serverNote) => serverNote.name === note.name,
                );
                if (noteMatch) {
                  console.log("notes match: ", noteMatch);
                  return {
                    ...noteMatch,
                  };
                } else {
                  console.log("no notes match");
                  return {
                    ...note,
                    state: "Terminated",
                  };
                }
              });
              return {
                ...localSym,
              };
            } else {
              localSym.state = "Stopped";
              localSym.notes = localSym.notes.map((note) => {
                note.state = "Terminated";
                return note;
              });
              return {
                ...localSym,
              };
            }
          });
        });
      } catch (error) {
        console.error("Error fetching symphonies:", error);
      }
    }

    // Initial fetch on mount
    fetchServerSymphonies();

    // Poll every 5 seconds, for example
    const intervalId = setInterval(fetchServerSymphonies, 5_000);
    return () => clearInterval(intervalId);
  }, []);

  // Function to add a new symphony to our tracked list
  const addSymphony = (newSymphony: Symphony) => {
    setTrackedSymphonies((prev) => {
      // If it already exists by name, decide how to handle (warn user, skip, or update).
      const exists = prev.some((s) => s.name === newSymphony.name);
      if (exists) {
        // In many apps, you'd handle duplicates more gracefully
        console.warn(`Symphony '${newSymphony.name}' is already tracked.`);
        return prev;
      }
      return [...prev, newSymphony];
    });
  };

  // Function to remove a symphony from our tracked list
  const removeSymphony = (name: string) => {
    setTrackedSymphonies((prev) => prev.filter((sym) => sym.name !== name));
  };

  const updateSymphony = (updatedSymphony: Symphony) => {
    setTrackedSymphonies((prev) => {
      const newList = prev.map((s) => {
        if (s.name === updatedSymphony.name) {
          return { ...updatedSymphony };
        }
        return s;
      });
      return [...newList];
    });
  };

  // The value we provide to our consumers
  const value: SymphoniesContextValue = {
    trackedSymphonies,
    addSymphony,
    removeSymphony,
    updateSymphony,
  };

  return (
    <SymphoniesContext.Provider value={value}>
      {children}
    </SymphoniesContext.Provider>
  );
};
