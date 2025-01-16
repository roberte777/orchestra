const axios = require("axios");

// Base URL of your API server
const BASE_URL = "http://localhost:3000";

// Function to start the symphony
async function getNotes() {
  try {
    const response = await axios.get(
      `${BASE_URL}/api/v1/notes`,
      {},
      {
        headers: {
          "Content-Type": "application/json",
        },
      },
    );

    console.log("Notes:", response.data);
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
getNotes();
