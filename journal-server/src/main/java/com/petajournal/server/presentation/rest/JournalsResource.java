package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.JournalService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.PageDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import com.petajournal.server.domain.repository.ItemRepository;

import java.time.LocalDate;
import java.util.*;

@Path("/api/journals")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class JournalsResource {

    @Inject
    JournalService journalService;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    ItemRepository itemRepository;

    private static final int MAX_TRANSCLUSION_DEPTH = 5;

    // --- Response records ---

    record GetOrCreateJournalResponse(PageDto page, List<ItemDto> items, boolean created, Map<String, List<ItemDto>> transcludedTrees) {}
    record GetMostRecentJournalBeforeResponse(boolean found, PageDto page, List<ItemDto> items, Map<String, List<ItemDto>> transcludedTrees) {}

    @POST
    @Path("/{name}")
    public GetOrCreateJournalResponse getOrCreateJournal(@PathParam("name") String name) {
        var result = journalService.getOrCreateJournalByName(name);
        var items = result.items();
        Map<String, List<ItemDto>> transcludedTrees = buildTranscludedTrees(items);
        return new GetOrCreateJournalResponse(
                DtoMapper.toDto(result.page()),
                loadItemDtos(items),
                result.created(),
                transcludedTrees);
    }

    @GET
    @Path("/before")
    public GetMostRecentJournalBeforeResponse getMostRecentJournalBefore(@QueryParam("date") String date) {
        LocalDate beforeDate = LocalDate.parse(date);
        var result = journalService.getMostRecentJournalBefore(beforeDate);
        if (!result.found()) {
            return new GetMostRecentJournalBeforeResponse(false, null, List.of(), Map.of());
        }
        var items = result.items();
        Map<String, List<ItemDto>> transcludedTrees = buildTranscludedTrees(items);
        return new GetMostRecentJournalBeforeResponse(
                true,
                DtoMapper.toDto(result.page()),
                loadItemDtos(items),
                transcludedTrees);
    }

    @GET
    @Path("/weekly/before")
    public GetMostRecentJournalBeforeResponse getMostRecentWeeklyJournalBefore(@QueryParam("week") String week) {
        var result = journalService.getMostRecentWeeklyJournalBefore(week);
        if (!result.found()) {
            return new GetMostRecentJournalBeforeResponse(false, null, List.of(), Map.of());
        }
        var items = result.items();
        Map<String, List<ItemDto>> transcludedTrees = buildTranscludedTrees(items);
        return new GetMostRecentJournalBeforeResponse(
                true,
                DtoMapper.toDto(result.page()),
                loadItemDtos(items),
                transcludedTrees);
    }

    private Map<String, List<ItemDto>> buildTranscludedTrees(List<Item> items) {
        Map<String, List<ItemDto>> trees = new LinkedHashMap<>();
        Set<String> visited = new HashSet<>();
        for (Item item : items) {
            if (item.transcludeItemId() != null) {
                String sourceId = item.transcludeItemId().toString();
                if (!trees.containsKey(sourceId)) {
                    loadTranscludedTree(sourceId, trees, visited, 0);
                }
            }
        }
        return trees;
    }

    private void loadTranscludedTree(String sourceId, Map<String, List<ItemDto>> trees, Set<String> visited, int depth) {
        if (depth >= MAX_TRANSCLUSION_DEPTH || visited.contains(sourceId)) return;
        visited.add(sourceId);

        List<Item> descendants = itemRepository.findItemWithDescendants(UUID.fromString(sourceId));
        if (descendants.isEmpty()) {
            trees.put(sourceId, List.of());
            return;
        }

        trees.put(sourceId, loadItemDtos(descendants));

        for (Item desc : descendants) {
            if (desc.transcludeItemId() != null) {
                String nestedSourceId = desc.transcludeItemId().toString();
                if (!trees.containsKey(nestedSourceId)) {
                    loadTranscludedTree(nestedSourceId, trees, visited, depth + 1);
                }
            }
        }
    }

    private List<ItemDto> loadItemDtos(List<Item> items) {
        List<String> ids = items.stream().map(i -> i.id().toString()).toList();
        var scalars = propertyRepository.findScalarByTargets("item", ids);
        var lists = propertyRepository.findListByTargets("item", ids);
        return DtoMapper.toDtos(items, scalars, lists);
    }
}
