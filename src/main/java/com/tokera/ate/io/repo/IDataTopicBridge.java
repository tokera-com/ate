package com.tokera.ate.io.repo;

import com.tokera.ate.dao.MessageBundle;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.dao.msg.MessageBase;
import com.tokera.ate.dao.msg.MessageSync;
import com.tokera.ate.dto.msg.MessageBaseDto;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.dto.msg.MessageSyncDto;
import com.tokera.ate.io.api.IPartitionKey;
import org.checkerframework.checker.nullness.qual.Nullable;

import javax.ws.rs.WebApplicationException;
import java.util.Set;
import java.util.UUID;

/**
 * Represents an interface that will stream data messages to and from a persistent storage (e.g. Kafka BUS or Local Data File)
 */
public interface IDataTopicBridge {

    IDataPartitionBridge createPartition(IPartitionKey key);
}
