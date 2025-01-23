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

export type NoteRestartPolicy = "Always" | "OnFailure" | "Never";

export interface TrackedNote {
  name: string;
  description: string;
  host: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  restart_policy: NoteRestartPolicy;
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
  getSymphony: (name: string) => TrackedSymphony | undefined;
  createSymphony: (sym: CrudSymphony) => Promise<void>;
  updateSymphony: (name: string, sym: CrudEditSymphony) => Promise<void>;
  removeSymphony: (name: string) => Promise<void>;
}

export interface CrudSymphony {
  name: string;
  notes: CrudNote[];
}

export interface CrudEditSymphony {
  name: string;
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

  // Load the user-tracked symphonies as they are updated
  useEffect(() => {
    const websocket = new WebSocket("/ws");

    websocket.onopen = () => {
      console.debug("WebSocket connection established");
    };

    websocket.onmessage = (event) => {
      const data: TrackedSymphony[] = JSON.parse(event.data);
      console.log(data);
      setTrackedSymphonies(data);
    };

    return () => {
      websocket.close();
    };
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
  };

  // Update an existing user-tracked symphony
  const updateSymphony = async (name: string, sym: CrudEditSymphony) => {
    const resp = await fetch(`/api/v1/tracked-symphonies/${name}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(sym),
    });
    if (!resp.ok) {
      console.error("Failed to update symphony:", resp.status);
      return;
    }
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
  };

  const getSymphony = (name: string) => {
    return trackedSymphonies.find((s) => s.name === name);
  };

  const value: SymphoniesContextValue = {
    trackedSymphonies,
    getSymphony,
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
