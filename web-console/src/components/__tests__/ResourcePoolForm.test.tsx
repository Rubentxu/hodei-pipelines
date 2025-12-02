import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { server } from "../../test/__mocks__/msw/server";
import { ResourcePoolForm } from "../ResourcePoolForm";

const mockNavigate = vi.fn();
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

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
    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

    expect(screen.getByText(/Create Resource Pool/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Pool Type/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Pool Name/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Minimum Size/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Maximum Size/)).toBeInTheDocument();
  });

  it("validates minimum size <= maximum size on client side", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

    const minSizeInput = screen.getByLabelText(/Minimum Size/);
    const maxSizeInput = screen.getByLabelText(/Maximum Size/);

    await user.clear(minSizeInput);
    await user.type(minSizeInput, "10");
    await user.clear(maxSizeInput);
    await user.type(maxSizeInput, "5");

    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    // Should show client-side validation error
    expect(
      screen.getByText(/Minimum size cannot be greater than maximum size/),
    ).toBeInTheDocument();
  });

  it("creates pool successfully with valid data", async () => {
    const user = userEvent.setup();

    server.use(
      http.post("/api/v1/worker-pools", async ({ request }) => {
        const body = (await request.json()) as any;
        return HttpResponse.json({
          id: "new-pool-id",
          ...body,
        });
      }),
    );

    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

    await user.type(screen.getByLabelText(/Pool Name/), "Test Pool");
    await user.type(screen.getByLabelText(/Minimum Size/), "1");
    await user.type(screen.getByLabelText(/Maximum Size/), "10");

    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith("/resource-pools");
    });
  });

  it("displays error message on creation failure", async () => {
    const user = userEvent.setup();

    server.use(
      http.post("/api/v1/worker-pools", async ({ request }) => {
        const body = (await request.json()) as any;
        if (body.name === "Existing") {
          return HttpResponse.json(
            { message: "Pool name already exists" },
            { status: 409 },
          );
        }
        return HttpResponse.json(mockPool);
      }),
    );

    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

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
    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

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

    server.use(http.post("/api/v1/worker-pools", () => new Promise(() => { })));

    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

    await user.type(screen.getByLabelText(/Pool Name/), "Test Pool");
    await user.click(screen.getByRole("button", { name: /Create Pool/ }));

    expect(screen.getByText(/Saving.../)).toBeInTheDocument();
  });

  it("displays required field validation", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <ResourcePoolForm />
      </MemoryRouter>,
    );

    const submitButton = screen.getByRole("button", { name: /Create Pool/ });
    await user.click(submitButton);

    expect(screen.getByText("Pool name is required")).toBeInTheDocument();
  });
});
