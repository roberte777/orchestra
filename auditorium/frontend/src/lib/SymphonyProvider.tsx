import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  PropsWithChildren,
} from "react";

// The shape we get from /api/v1/tracked-symphonies
// (Make sure it matches what your Rust code returns!)
export type AuditoriumResourceState = "Running" | "Stopped";

export type NoteState =
  | "Pending"
  | "Running"
  | "Terminated"
  | "Completed"
  | "Crashed";

export interface TrackedNote {
  name: string;
  description: string;
  host: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  restart_policy: string;
  state: NoteState;
  auditorium_state: AuditoriumResourceState;
}

export interface TrackedSymphony {
  name: string;
  notes: TrackedNote[];
  auditorium_state: AuditoriumResourceState;
}

// The shape of our Context
export interface SymphoniesContextValue {
  trackedSymphonies: TrackedSymphony[];
  reloadSymphonies: () => void;
  createSymphony: (sym: CrudSymphony) => Promise<void>;
  updateSymphony: (name: string, sym: CrudSymphony) => Promise<void>;
  removeSymphony: (name: string) => Promise<void>;
}

export interface CrudSymphony {
  name: string;
  notes: CrudNote[];
}

export interface CrudNote {
  name: string;
  description: string;
  host: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  restart_policy: string;
}

// Create the context
const SymphoniesContext = createContext<SymphoniesContextValue | undefined>(
  undefined,
);

export const useSymphonies = (): SymphoniesContextValue => {
  const ctx = useContext(SymphoniesContext);
  if (!ctx) {
    throw new Error("useSymphonies must be used within a SymphoniesProvider");
  }
  return ctx;
};

export const SymphoniesProvider: React.FC<PropsWithChildren> = ({
  children,
}) => {
  const [trackedSymphonies, setTrackedSymphonies] = useState<TrackedSymphony[]>(
    [],
  );

  // A function to load/poll data from the auditorium
  const reloadSymphonies = async () => {
    try {
      const resp = await fetch("/api/v1/tracked-symphonies");
      if (!resp.ok) {
        console.error("Failed to fetch tracked symphonies");
        return;
      }
      const data: TrackedSymphony[] = await resp.json();
      setTrackedSymphonies(data);
    } catch (error) {
      console.error("Error fetching symphonies:", error);
    }
  };

  // On mount, load once. Then poll every 5s.
  useEffect(() => {
    reloadSymphonies();
    const interval = setInterval(() => {
      reloadSymphonies();
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  // Create a new user-tracked symphony
  const createSymphony = async (sym: CrudSymphony) => {
    const resp = await fetch("/api/v1/tracked-symphonies", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(sym),
    });
    if (!resp.ok) {
      console.error("Failed to create symphony:", resp.status);
      return;
    }
    await reloadSymphonies();
  };

  // Update an existing user-tracked symphony
  const updateSymphony = async (name: string, sym: CrudSymphony) => {
    const resp = await fetch(`/api/v1/tracked-symphonies/${name}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(sym),
    });
    if (!resp.ok) {
      console.error("Failed to update symphony:", resp.status);
      return;
    }
    await reloadSymphonies();
  };

  // Remove a user-tracked symphony
  const removeSymphony = async (name: string) => {
    const resp = await fetch(`/api/v1/tracked-symphonies/${name}`, {
      method: "DELETE",
    });
    if (!resp.ok) {
      console.error("Failed to remove symphony:", resp.status);
      return;
    }
    await reloadSymphonies();
  };

  const value: SymphoniesContextValue = {
    trackedSymphonies,
    reloadSymphonies,
    createSymphony,
    updateSymphony,
    removeSymphony,
  };

  return (
    <SymphoniesContext.Provider value={value}>
      {children}
    </SymphoniesContext.Provider>
  );
};
