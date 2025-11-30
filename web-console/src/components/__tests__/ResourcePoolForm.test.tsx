import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ResourcePoolForm } from "../ResourcePoolForm";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";

const mockPool = {
  id: "new-pool-id",
  name: "Test Pool",
  poolType: "Docker" as const,
  providerName: "docker",
  minSize: 1,
  maxSize: 10,
  defaultResources: {
    cpu_m: 2000,
    memory_mb: 4096,
    gpu: null,
  },
  tags: { env: "test" },
};

describe("ResourcePoolForm", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("renders form correctly in create mode", () => {
    render(<ResourcePoolForm />);

    expect(screen.getByText(/Create Resource Pool/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Pool Type/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Pool Name/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Minimum Size/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Maximum Size/)).toBeInTheDocument();
  });

  it("validates minimum size <= maximum size on client side", async () => {
    const user = userEvent.setup();
    render(<ResourcePoolForm />);

    const minSizeInput = screen.getByLabelText(/Minimum Size/);
    const maxSizeInput = screen.getByLabelText(/Maximum Size/);

    await user.clear(minSizeInput);
    await user.type(minSizeInput, "10");
    await user.clear(maxSizeInput);
    await user.type(maxSizeInput, "5");

    // Should show client-side validation error
    expect(
      screen.getByText(/minimum size cannot be greater than maximum size/),
    ).toBeInTheDocument();
  });

  it("creates pool successfully with valid data", async () => {
    const user = userEvent.setup();

    server.use(
      http.post("/api/v1/worker-pools", async ({ request }) => {
        const body = await request.json();
        return HttpResponse.json({
          id: "new-pool-id",
          ...body,
        });
      }),
    );

    render(<ResourcePoolForm />);

    await user.type(screen.getByLabelText(/Pool Name/), "Test Pool");
    await user.type(screen.getByLabelText(/Minimum Size/), "1");
    await user.type(screen.getByLabelText(/Maximum Size/), "10");

    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    await waitFor(() => {
      expect(screen.getByText(/Saved!/)).toBeInTheDocument();
    });
  });

  it("displays error message on creation failure", async () => {
    const user = userEvent.setup();

    server.use(
      http.post("/api/v1/worker-pools", async ({ request }) => {
        const body = await request.json();
        if (body.name === "Existing") {
          return HttpResponse.json(
            { message: "Pool name already exists" },
            { status: 409 },
          );
        }
        return HttpResponse.json(mockPool);
      }),
    );

    render(<ResourcePoolForm />);

    await user.type(screen.getByLabelText(/Pool Name/), "Existing");
    await user.type(screen.getByLabelText(/Minimum Size/), "1");
    await user.type(screen.getByLabelText(/Maximum Size/), "10");
    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    await waitFor(() => {
      expect(screen.getByText(/Pool name already exists/)).toBeInTheDocument();
    });
  });

  it("adds and removes tags correctly", async () => {
    const user = userEvent.setup();
    render(<ResourcePoolForm />);

    const keyInput = screen.getByPlaceholderText("Key");
    const valueInput = screen.getByPlaceholderText("Value");
    const addButton = screen.getByText("Add");

    await user.type(keyInput, "env");
    await user.type(valueInput, "test");
    await user.click(addButton);

    expect(screen.getByText(/env: test/)).toBeInTheDocument();

    const removeButton = screen.getByText("×");
    await user.click(removeButton);

    await waitFor(() => {
      expect(screen.queryByText(/env: test/)).not.toBeInTheDocument();
    });
  });

  it("displays loading state during save", async () => {
    const user = userEvent.setup();

    server.use(http.post("/api/v1/worker-pools", () => new Promise(() => {})));

    render(<ResourcePoolForm />);

    await user.type(screen.getByLabelText(/Pool Name/), "Test Pool");
    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    expect(screen.getByText(/Saving.../)).toBeInTheDocument();
  });

  it("displays required field validation", async () => {
    const user = userEvent.setup();
    render(<ResourcePoolForm />);

    const submitButton = screen.getByRole("button", { name: /Create Pool/ });
    await user.click(submitButton);

    expect(screen.getByText("Pool name is required")).toBeInTheDocument();
  });
});
