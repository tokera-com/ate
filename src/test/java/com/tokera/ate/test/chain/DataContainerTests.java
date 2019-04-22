package com.tokera.ate.test.chain;

import com.tokera.ate.common.LoggerHook;
import com.tokera.ate.dao.filters.ResourceScopeInterceptor;
import com.tokera.ate.delegates.LoggingDelegate;
import com.tokera.ate.dto.msg.MessageDataDto;
import com.tokera.ate.dto.msg.MessageDataHeaderDto;
import com.tokera.ate.dto.msg.MessageDataMetaDto;
import com.tokera.ate.dto.msg.MessageMetaDto;
import com.tokera.ate.extensions.DaoParentDiscoveryExtension;
import com.tokera.ate.extensions.StartupBeanExtension;
import com.tokera.ate.extensions.YamlTagDiscoveryExtension;
import com.tokera.ate.io.MemoryCacheIO;
import com.tokera.ate.io.core.StorageSystemFactory;
import com.tokera.ate.io.repo.DataContainer;
import com.tokera.ate.io.repo.DataGraphNode;
import com.tokera.ate.kafka.KafkaConfigTools;
import com.tokera.ate.security.EncryptKeyCachePerRequest;
import com.tokera.ate.test.dao.MyAccount;
import org.jboss.weld.bootstrap.spi.BeanDiscoveryMode;
import org.jboss.weld.environment.se.Weld;
import org.jboss.weld.junit5.WeldInitiator;
import org.jboss.weld.junit5.WeldJunit5Extension;
import org.jboss.weld.junit5.WeldSetup;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.LinkedList;
import java.util.UUID;

@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class DataContainerTests {

    @Test
    public void emptyContainer() {

        DataContainer container = new DataContainer();
        assert container.getLastHeaderOrNull() == null;
        assert container.hasPayload() == false;
        assert container.getLastOrNull() == null;
        assert container.getLastDataOrNull() == null;
        assert container.getLastOffsetOrNull() == null;
    }

    @Test
    public void soloContainer() {
        DataContainer container = new DataContainer();

        MessageDataHeaderDto header = new MessageDataHeaderDto(UUID.randomUUID(), UUID.randomUUID(), null, MyAccount.class.getSimpleName());
        MessageDataDto data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        assert container.getLastHeaderOrNull() != null;
        assert container.hasPayload() == false;
        assert container.getLastOrNull() != null;
        assert container.getLastDataOrNull() != null;
        assert container.getLastOffsetOrNull() != null;
        assert container.leaves.size() == 1;
    }

    @Test
    public void linearContainer() {
        DataContainer container = new DataContainer();
        UUID version0 = UUID.randomUUID();
        UUID version1 = UUID.randomUUID();
        UUID version2 = UUID.randomUUID();
        UUID version3 = UUID.randomUUID();

        MessageDataHeaderDto header = new MessageDataHeaderDto(UUID.randomUUID(), version1, version0, MyAccount.class.getSimpleName());
        MessageDataDto data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version2, version1, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version3, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        LinkedList<DataGraphNode> scope = container.leaves;
        assert scope.size() == 1;
        assert scope.get(0).version.compareTo(version3) == 0;
    }

    @Test
    public void triMergeContainer() {
        DataContainer container = new DataContainer();
        UUID version0 = UUID.randomUUID();
        UUID version1 = UUID.randomUUID();
        UUID version2 = UUID.randomUUID();
        UUID version3a = UUID.randomUUID();
        UUID version3b = UUID.randomUUID();

        MessageDataHeaderDto header = new MessageDataHeaderDto(UUID.randomUUID(), version1, version0, MyAccount.class.getSimpleName());
        MessageDataDto data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version2, version1, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version3a, version2,MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version3b,version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        LinkedList<DataGraphNode> scope = container.leaves;
        assert scope.size() == 2;
        assert scope.get(0).version.compareTo(version3a) == 0;
        assert scope.get(1).version.compareTo(version3b) == 0;
    }

    @Test
    public void quad1MergeContainer() {
        DataContainer container = new DataContainer();
        UUID version0 = UUID.randomUUID();
        UUID version1 = UUID.randomUUID();
        UUID version2 = UUID.randomUUID();
        UUID version3 = UUID.randomUUID();
        UUID version4 = UUID.randomUUID();
        UUID version4b = UUID.randomUUID();

        MessageDataHeaderDto header = new MessageDataHeaderDto(UUID.randomUUID(), version1, version0, MyAccount.class.getSimpleName());
        MessageDataDto data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version2, version1, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version3, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version4, version3, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version4b, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        LinkedList<DataGraphNode> scope = container.leaves;
        assert scope.size() == 2;
        assert scope.get(0).version.compareTo(version4) == 0;
        assert scope.get(1).version.compareTo(version4b) == 0;
    }

    @Test
    public void quad2MergeContainer() {
        DataContainer container = new DataContainer();
        UUID version0 = UUID.randomUUID();
        UUID version1 = UUID.randomUUID();
        UUID version2 = UUID.randomUUID();
        UUID version3 = UUID.randomUUID();
        UUID version4 = UUID.randomUUID();
        UUID version4b = UUID.randomUUID();

        MessageDataHeaderDto header = new MessageDataHeaderDto(UUID.randomUUID(), version1, version0, MyAccount.class.getSimpleName());
        MessageDataDto data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version2, version1, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version3, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version4, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        header = new MessageDataHeaderDto(UUID.randomUUID(), version4b, version2, MyAccount.class.getSimpleName());
        data = new MessageDataDto(header, null, null) ;
        container.add(data, new MessageMetaDto(0,0,0));

        LinkedList<DataGraphNode> scope = container.leaves;
        assert scope.size() == 3;
        assert scope.get(0).version.compareTo(version3) == 0;
        assert scope.get(1).version.compareTo(version4) == 0;
        assert scope.get(2).version.compareTo(version4b) == 0;
    }
}
