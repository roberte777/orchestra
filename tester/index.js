const axios = require("axios");

// Base URL of your API server
const BASE_URL = "http://localhost:3000";

// Symphony data to send in the POST request
const symphonyData = {
  name: "Symphony1",
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
      restart_policy: "never",
    },
    {
      name: "Note2",
      description: "Second note",
      host: "me",
      command: "ping",
      args: ["-c", "4", "127.0.0.1"],
      env: {
        VAR1: "value1",
      },
      restart_policy: "on_failure",
    },
  ],
};

// Function to start the symphony
async function startSymphony() {
  try {
    const response = await axios.post(
      `${BASE_URL}/api/v1/symphonies`,
      symphonyData,
      {
        headers: {
          "Content-Type": "application/json",
        },
      },
    );

    console.log("Symphony started successfully:", response.data);
  } catch (error) {
    if (error.response) {
      console.error("Error response from server:", error.response.data);
    } else if (error.request) {
      console.error("No response received from server:", error.request);
    } else {
      console.error("Error setting up request:", error.message);
    }
  }
}

// Run the function
startSymphony();
